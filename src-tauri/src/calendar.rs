//! Read-only Google Calendar connection. OAuth tokens use the existing credential adapter.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{Duration as Days, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{sync::Mutex, time::Duration};
use tauri::{AppHandle, Manager};
use tauri_plugin_shell::ShellExt;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpListener};
use tokio_util::sync::CancellationToken;
use crate::state::AppState;

const KEY: &str = "google_calendar_oauth";
const SCOPE: &str = "https://www.googleapis.com/auth/calendar.events.readonly";
#[derive(Default)]
pub struct CalendarState {
    operation: tokio::sync::Mutex<()>,
    pending: Mutex<Option<CancellationToken>>,
}
#[derive(Serialize, Deserialize)]
struct Account {
    client_id: String,
    client_secret: String,
    access_token: String,
    refresh_token: String,
    expires_at: i64,
}
async fn stored(app: &AppHandle, write: Option<Option<String>>) -> Result<Option<String>, String> {
    let credentials = app.state::<AppState>().credentials.clone().ok_or("Credential storage unavailable")?;
    tokio::task::spawn_blocking(move || {
        let manager = credentials.lock().map_err(|_| "Credential storage unavailable")?;
        match write {
            None => manager.get_key(KEY),
            Some(Some(value)) => { manager.store_key(KEY, &value)?; Ok(None) },
            Some(None) => { manager.delete_key(KEY)?; Ok(None) },
        }
    }).await.map_err(|_| "Credential operation failed")?
}
fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder().timeout(Duration::from_secs(25)).build().map_err(|_| "Calendar connection unavailable".into())
}
async fn token_request(form: &[(&str, &str)]) -> Result<Value, String> {
    let response = client()?.post("https://oauth2.googleapis.com/token").form(form).send().await
        .map_err(|_| "Could not reach Google. Check your connection and retry.")?;
    let status = response.status();
    let body: Value = response.json().await.map_err(|_| "Google returned an unreadable response")?;
    if !status.is_success() {
        return Err(match body["error"].as_str() {
            Some("invalid_grant") => "Google authorization expired or was revoked. Connect Google Calendar again.".into(),
            Some("invalid_client") => "Check your Google Desktop app OAuth client ID and client secret.".into(),
            _ => format!("Google authorization failed (HTTP {}). Try connecting again.", status.as_u16()),
        });
    }
    Ok(body)
}
fn apply_token(account: &mut Account, body: &Value) -> Result<(), String> {
    account.access_token = body["access_token"].as_str().filter(|s| !s.is_empty()).ok_or("Google did not return an access token")?.into();
    if let Some(refresh) = body["refresh_token"].as_str() { account.refresh_token = refresh.into(); }
    account.expires_at = Utc::now().timestamp() + body["expires_in"].as_i64().unwrap_or(3600).clamp(1, 86400);
    Ok(())
}
fn callback_code(target: &str, expected_state: &str) -> Result<String, String> {
    let url = url::Url::parse(&format!("http://127.0.0.1{target}")).map_err(|_| "Invalid callback")?;
    if url.path() != "/oauth/calendar" { return Err("Invalid callback path".into()); }
    let pairs: Vec<_> = url.query_pairs().collect();
    if pairs.iter().filter(|(k, _)| k == "state").count() != 1 ||
        !pairs.iter().any(|(k,v)| k == "state" && v == expected_state) { return Err("Authorization state mismatch".into()); }
    if pairs.iter().any(|(k,_)| k == "error") { return Err("Google Calendar permission was not granted. You can reconnect when ready.".into()); }
    pairs.iter().find(|(k,_)| k == "code").map(|(_,v)| v.to_string()).filter(|v| !v.is_empty()).ok_or("Missing authorization code".into())
}
async fn wait_callback(listener: TcpListener, state: &str) -> Result<String, String> {
    loop {
        let (mut socket, _) = listener.accept().await.map_err(|_| "Could not receive Google sign-in")?;
        let mut buffer = Vec::new();
        let read = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let mut part = [0u8; 1024];
                let n = socket.read(&mut part).await?;
                if n == 0 { break; }
                buffer.extend_from_slice(&part[..n]);
                if buffer.windows(4).any(|w| w == b"\r\n\r\n") || buffer.len() >= 16384 { break; }
            }
            Ok::<(), std::io::Error>(())
        }).await;
        if !matches!(read, Ok(Ok(()))) { continue; }
        let request = String::from_utf8_lossy(&buffer);
        let target = request.lines().next().and_then(|l| l.strip_prefix("GET ")).and_then(|l| l.split_whitespace().next()).unwrap_or("");
        let result = callback_code(target, state);
        let accepted = result.is_ok();
        let denied = result.as_ref().err().is_some_and(|e| e.starts_with("Google Calendar permission"));
        let message = if accepted { "Sign-in received. Return to KebabM to finish connecting." } else { "Sign-in was not accepted. Return to KebabM and try again." };
        let reply = format!("HTTP/1.1 {}\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}", if accepted { "200 OK" } else { "400 Bad Request" }, message.len(), message);
        let _ = tokio::time::timeout(Duration::from_secs(2), socket.write_all(reply.as_bytes())).await;
        if accepted || denied { return result; }
    }
}
#[tauri::command]
pub async fn calendar_status(app: AppHandle) -> Result<Value, String> {
    let raw = stored(&app, None).await?;
    let account = raw.map(|s| serde_json::from_str::<Account>(&s).map_err(|_| "Calendar credentials need to be reconnected")).transpose()?;
    Ok(json!({"connected": account.is_some(), "clientId": account.map(|a| a.client_id), "scope": "Read-only calendar events"}))
}
#[tauri::command]
pub async fn calendar_connect(app: AppHandle, client_id: String, client_secret: Option<String>) -> Result<(), String> {
    let client_id = client_id.trim().to_string();
    if !client_id.ends_with(".apps.googleusercontent.com") || client_id.len() > 256 { return Err("Enter a Desktop app OAuth client ID from Google Cloud. An AI Studio API key cannot connect Calendar.".into()); }
    let service = app.state::<CalendarState>();
    let _guard = service.operation.try_lock().map_err(|_| "Another calendar operation is running. Try again shortly.")?;
    let cancel = CancellationToken::new();
    *service.pending.lock().map_err(|_| "Calendar unavailable")? = Some(cancel.clone());
    let result = async {
        let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|_| "Could not open the local Google sign-in callback")?;
        let redirect = format!("http://127.0.0.1:{}/oauth/calendar", listener.local_addr().map_err(|_| "Callback unavailable")?.port());
        let verifier = format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());
        let state = uuid::Uuid::new_v4().to_string();
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        let mut authorize = url::Url::parse("https://accounts.google.com/o/oauth2/v2/auth").unwrap();
        authorize.query_pairs_mut().extend_pairs([
            ("client_id", client_id.as_str()), ("redirect_uri", &redirect), ("response_type", "code"),
            ("scope", SCOPE), ("access_type", "offline"), ("prompt", "consent"),
            ("code_challenge", &challenge), ("code_challenge_method", "S256"), ("state", &state),
        ]);
        #[allow(deprecated)]
        app.shell().open(authorize.to_string(), None).map_err(|_| "Could not open your browser for Google sign-in")?;
        let code = tokio::select! {
            _ = cancel.cancelled() => return Err("Calendar connection cancelled".into()),
            result = tokio::time::timeout(Duration::from_secs(180), wait_callback(listener, &state)) => result.map_err(|_| "Google sign-in timed out. Try connecting again.")??,
        };
        let mut account = Account { client_id, client_secret: client_secret.unwrap_or_default(), access_token: String::new(), refresh_token: String::new(), expires_at: 0 };
        let body = token_request(&[("client_id", &account.client_id), ("client_secret", &account.client_secret), ("code", &code), ("redirect_uri", &redirect), ("code_verifier", &verifier), ("grant_type", "authorization_code")]).await?;
        apply_token(&mut account, &body)?;
        if account.refresh_token.is_empty() { return Err("Google did not grant offline access. Connect again and allow Calendar access.".into()); }
        if cancel.is_cancelled() { return Err("Calendar connection cancelled".into()); }
        stored(&app, Some(Some(serde_json::to_string(&account).map_err(|_| "Could not save Calendar connection")?))).await?;
        Ok(())
    }.await;
    *service.pending.lock().map_err(|_| "Calendar unavailable")? = None;
    result
}
#[tauri::command]
pub fn calendar_cancel(app: AppHandle) {
    if let Ok(pending) = app.state::<CalendarState>().pending.lock() { if let Some(cancel) = pending.as_ref() { cancel.cancel(); } }
}
#[tauri::command]
pub async fn calendar_disconnect(app: AppHandle) -> Result<(), String> {
    calendar_cancel(app.clone());
    let service = app.state::<CalendarState>();
    let _guard = service.operation.lock().await;
    // Local disconnect only; Google Account permissions can also be revoked by the user.
    stored(&app, Some(None)).await?;
    Ok(())
}
fn event_view(event: &Value) -> Option<Value> {
    if event["status"] == "cancelled" || event["attendees"].as_array().is_some_and(|people| people.iter().any(|p| p["self"] == true && p["responseStatus"] == "declined")) { return None; }
    let start = event["start"]["dateTime"].as_str().or(event["start"]["date"].as_str())?;
    let end = event["end"]["dateTime"].as_str().or(event["end"]["date"].as_str()).unwrap_or(start);
    let conference = event["hangoutLink"].as_str().or_else(|| event["conferenceData"]["entryPoints"].as_array()?.iter().find(|p| p["entryPointType"] == "video")?["uri"].as_str());
    let safe_url = |raw: Option<&str>| raw.and_then(|s| url::Url::parse(s).ok()).filter(|u| u.scheme() == "https" && u.username().is_empty() && u.password().is_none()).map(|u| u.to_string());
    Some(json!({"id": event["id"], "title": event["summary"].as_str().unwrap_or("Untitled meeting"), "start": start, "end": end,
        "allDay": event["start"]["dateTime"].is_null(), "joinUrl": safe_url(conference), "htmlUrl": safe_url(event["htmlLink"].as_str()),
        "attendeeCount": event["attendees"].as_array().map_or(0, |a| a.len()) }))
}
#[tauri::command]
pub async fn calendar_events(app: AppHandle) -> Result<Value, String> {
    let service = app.state::<CalendarState>();
    let _guard = service.operation.try_lock().map_err(|_| "Calendar is connecting or refreshing. Try again shortly.")?;
    let raw = stored(&app, None).await?.ok_or("Connect Google Calendar to see upcoming meetings")?;
    let mut account: Account = serde_json::from_str(&raw).map_err(|_| "Reconnect Google Calendar")?;
    if account.expires_at <= Utc::now().timestamp() + 60 {
        let body = token_request(&[("client_id", &account.client_id), ("client_secret", &account.client_secret), ("refresh_token", &account.refresh_token), ("grant_type", "refresh_token")]).await?;
        apply_token(&mut account, &body)?;
        stored(&app, Some(Some(serde_json::to_string(&account).map_err(|_| "Could not save Calendar connection")?))).await?;
    }
    let now = Utc::now();
    let mut events = Vec::new();
    let mut page = String::new();
    for _ in 0..10 {
        let mut query = vec![("timeMin", now.to_rfc3339()), ("timeMax", (now + Days::days(7)).to_rfc3339()), ("singleEvents", "true".into()), ("orderBy", "startTime".into()), ("maxResults", "250".into())];
        if !page.is_empty() { query.push(("pageToken", page)); }
        let response = client()?.get("https://www.googleapis.com/calendar/v3/calendars/primary/events").bearer_auth(&account.access_token).query(&query).send().await.map_err(|_| "Calendar could not refresh. Check your connection and retry.")?;
        if !response.status().is_success() { return Err(match response.status().as_u16() {
            401 => "Google access has expired. Reconnect your calendar.".into(),
            403 => "Google Calendar access is denied. Enable the Calendar API in your OAuth project and grant calendar access.".into(),
            429 => "Google Calendar is rate limiting requests. Try refreshing later.".into(),
            code => format!("Google Calendar is unavailable (HTTP {code}). Try again shortly."),
        }); }
        let body: Value = response.json().await.map_err(|_| "Google returned an unreadable calendar")?;
        let items = body["items"].as_array().ok_or("Google returned an invalid events list")?;
        events.extend(items.iter().filter_map(event_view));
        page = body["nextPageToken"].as_str().unwrap_or_default().into();
        if page.is_empty() { return Ok(json!({"events": events, "syncedAt": Utc::now().to_rfc3339(), "truncated": false})); }
    }
    Ok(json!({"events": events, "syncedAt": Utc::now().to_rfc3339(), "truncated": true}))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn callback_requires_exact_state_and_path() {
        assert_eq!(callback_code("/oauth/calendar?state=abc&code=hello", "abc").unwrap(), "hello");
        for path in ["/oauth/calendar?state=abc-extra&code=x", "/?state=abc&code=x", "/oauth/calendar?state=abc&state=x&code=x"] { assert!(callback_code(path, "abc").is_err()); }
        assert!(callback_code("/oauth/calendar?state=abc&error=access_denied", "abc").unwrap_err().contains("not granted"));
    }
    #[test]
    fn events_preserve_all_day_dates_and_reject_unsafe_links() {
        let event = json!({"id":"a", "start":{"date":"2026-09-28"}, "end":{"date":"2026-09-29"}, "hangoutLink":"javascript:alert(1)"});
        let row = event_view(&event).unwrap();
        assert_eq!(row["allDay"], true); assert_eq!(row["start"], "2026-09-28"); assert!(row["joinUrl"].is_null());
        assert!(event_view(&json!({"status":"cancelled"})).is_none());
        assert!(event_view(&json!({"attendees":[{"self":true,"responseStatus":"declined"}]})).is_none());
    }
    #[tokio::test]
    async fn callback_handles_fragmented_input_and_ignores_wrong_state() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move { wait_callback(listener, "expected").await });
        let mut invalid = tokio::net::TcpStream::connect(address).await.unwrap();
        invalid.write_all(b"GET /oauth/calendar?state=wrong&code=bad HTTP/1.1\r\nHost: localhost\r\n\r\n").await.unwrap();
        let mut reply = String::new(); invalid.read_to_string(&mut reply).await.unwrap();
        assert!(reply.starts_with("HTTP/1.1 400"));
        let mut valid = tokio::net::TcpStream::connect(address).await.unwrap();
        valid.write_all(b"GET /oauth/calendar?state=expected&code=").await.unwrap();
        tokio::task::yield_now().await;
        valid.write_all(b"good HTTP/1.1\r\nHost: localhost\r\n\r\n").await.unwrap();
        assert_eq!(tokio::time::timeout(Duration::from_secs(3), task).await.unwrap().unwrap().unwrap(), "good");
    }
    #[tokio::test]
    async fn cancelled_callback_releases_listener() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let cancel = CancellationToken::new(); cancel.cancel();
        tokio::select! {
            _ = cancel.cancelled() => {},
            _ = wait_callback(listener, "expected") => panic!("callback should be cancelled"),
        }
        assert!(TcpListener::bind(address).await.is_ok());
    }
    #[test]
    fn refresh_keeps_existing_refresh_token() {
        let mut account = Account { client_id:String::new(), client_secret:String::new(), access_token:String::new(), refresh_token:"existing".into(), expires_at:0 };
        apply_token(&mut account, &json!({"access_token":"new", "expires_in":3600})).unwrap();
        assert_eq!(account.refresh_token, "existing");
        assert!(apply_token(&mut account, &json!({})).is_err());
    }
}
