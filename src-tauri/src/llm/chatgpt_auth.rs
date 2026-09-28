use base64::Engine;
use tauri::Emitter;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::time::Duration;

pub const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
pub const ISSUER: &str = "https://auth.openai.com";
pub const CODEX_API: &str = "https://chatgpt.com/backend-api/codex";

const ACCESS_KEY: &str = "chatgpt_access_token";
const REFRESH_KEY: &str = "chatgpt_refresh_token";
const ACCOUNT_KEY: &str = "chatgpt_account_id";

#[derive(Debug, Clone)]
pub struct ChatGptSession {
    pub access_token: String,
    pub refresh_token: String,
    pub account_id: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
}

pub fn store_session(
    credentials: &crate::credentials::CredentialManager,
    session: &ChatGptSession,
) -> Result<(), String> {
    credentials.store_key(ACCESS_KEY, &session.access_token)?;
    credentials.store_key(REFRESH_KEY, &session.refresh_token)?;
    if !session.account_id.is_empty() {
        credentials.store_key(ACCOUNT_KEY, &session.account_id)?;
    }
    Ok(())
}

pub fn load_session(
    credentials: &crate::credentials::CredentialManager,
) -> Result<Option<ChatGptSession>, String> {
    let access = credentials.get_key(ACCESS_KEY)?;
    let refresh = credentials.get_key(REFRESH_KEY)?;
    let Some(access_token) = access else {
        return Ok(None);
    };
    let Some(refresh_token) = refresh else {
        return Ok(None);
    };
    let account_id = credentials.get_key(ACCOUNT_KEY)?.unwrap_or_default();
    Ok(Some(ChatGptSession {
        access_token,
        refresh_token,
        account_id,
    }))
}

pub async fn sign_in_with_browser(app: &tauri::AppHandle) -> Result<ChatGptSession, String> {
    let verifier = random_token(32);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let state = random_token(16);

    let listener = bind_callback().await?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let redirect_uri = format!("http://localhost:{port}/auth/callback");
    let url = authorize_url(&redirect_uri, &challenge, &state);

    let _ = app.emit(
        "chatgpt-login-url",
        serde_json::json!({ "url": url, "port": port }),
    );

    let (code, returned_state) = wait_for_callback(listener).await?;
    if returned_state != state && !returned_state.starts_with(&state) {
        return Err("ChatGPT login state did not match. Start the sign-in again.".to_string());
    }

    exchange_code(&redirect_uri, &verifier, &code).await
}

fn authorize_url(redirect_uri: &str, challenge: &str, state: &str) -> String {
    let mut url = url::Url::parse(&format!("{ISSUER}/oauth/authorize")).expect("issuer url");
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("response_type", "code");
        q.append_pair("client_id", CLIENT_ID);
        q.append_pair("redirect_uri", redirect_uri);
        q.append_pair(
            "scope",
            "openid profile email offline_access api.connectors.read api.connectors.invoke",
        );
        q.append_pair("code_challenge", challenge);
        q.append_pair("code_challenge_method", "S256");
        q.append_pair("state", state);
        q.append_pair("id_token_add_organizations", "true");
        q.append_pair("codex_cli_simplified_flow", "true");
        q.append_pair("originator", "nexq");
    }
    url.to_string()
}

async fn bind_callback() -> Result<tokio::net::TcpListener, String> {
    match tokio::net::TcpListener::bind("127.0.0.1:1455").await {
        Ok(listener) => Ok(listener),
        Err(_) => tokio::net::TcpListener::bind("127.0.0.1:1457")
            .await
            .map_err(|_| {
                "ChatGPT login needs localhost port 1455 or 1457, and both are in use.".to_string()
            }),
    }
}

async fn wait_for_callback(listener: tokio::net::TcpListener) -> Result<(String, String), String> {
    let accept = tokio::time::timeout(Duration::from_secs(180), listener.accept()).await;
    let (mut socket, _) = accept
        .map_err(|_| "ChatGPT sign-in timed out. Open the link and finish login.".to_string())?
        .map_err(|e| e.to_string())?;

    let mut buf = vec![0u8; 8192];
    let n = tokio::time::timeout(Duration::from_secs(10), socket.read(&mut buf))
        .await
        .map_err(|_| "Login callback did not send a request".to_string())?
        .map_err(|e| e.to_string())?;
    let request = String::from_utf8_lossy(&buf[..n]).to_string();
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");

    let page = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n<!doctype html><title>ZaiqoM-MeetingHelper</title><p>Signed in with ChatGPT. You can close this window.</p>";
    let _ = socket.write_all(page.as_bytes()).await;

    let parsed = url::Url::parse(&format!("http://localhost{path}")).map_err(|e| e.to_string())?;
    let code = parsed
        .query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.to_string())
        .ok_or_else(|| "ChatGPT did not return a login code".to_string())?;
    let returned_state = parsed
        .query_pairs()
        .find(|(k, _)| k == "state")
        .map(|(_, v)| v.to_string())
        .unwrap_or_default();
    Ok((code, returned_state))
}

async fn exchange_code(
    redirect_uri: &str,
    verifier: &str,
    code: &str,
) -> Result<ChatGptSession, String> {
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{ISSUER}/oauth/token"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!(
            "grant_type=authorization_code&client_id={}&code={}&redirect_uri={}&code_verifier={}",
            form_encode(CLIENT_ID),
            form_encode(code),
            form_encode(redirect_uri),
            form_encode(verifier)
        ))
        .send()
        .await
        .map_err(|e| format!("ChatGPT token exchange failed: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("ChatGPT token exchange failed ({status}): {body}"));
    }
    let tokens: TokenResponse = response
        .json()
        .await
        .map_err(|e| format!("ChatGPT token response was not valid: {e}"))?;
    Ok(session_from_tokens(tokens))
}

pub async fn refresh_session(session: &ChatGptSession) -> Result<ChatGptSession, String> {
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{ISSUER}/oauth/token"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!(
            "grant_type=refresh_token&client_id={}&refresh_token={}",
            form_encode(CLIENT_ID),
            form_encode(&session.refresh_token)
        ))
        .send()
        .await
        .map_err(|e| format!("ChatGPT refresh failed: {e}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("ChatGPT refresh failed ({status}): {body}"));
    }
    let tokens: TokenResponse = response.json().await.map_err(|e| e.to_string())?;
    let mut next = session_from_tokens(tokens);
    if next.refresh_token.is_empty() {
        next.refresh_token = session.refresh_token.clone();
    }
    if next.account_id.is_empty() {
        next.account_id = session.account_id.clone();
    }
    Ok(next)
}

fn session_from_tokens(tokens: TokenResponse) -> ChatGptSession {
    let account_id = tokens
        .id_token
        .as_deref()
        .and_then(account_id_from_jwt)
        .or_else(|| account_id_from_jwt(&tokens.access_token))
        .unwrap_or_default();
    ChatGptSession {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token.unwrap_or_default(),
        account_id,
    }
}

fn account_id_from_jwt(token: &str) -> Option<String> {
    let payload = token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    value
        .get("chatgpt_account_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            value
                .get("https://api.openai.com/auth")
                .and_then(|v| v.get("chatgpt_account_id"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
}

fn form_encode(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

fn random_token(bytes: usize) -> String {
    let mut raw = String::new();
    while raw.len() < bytes {
        raw.push_str(&uuid::Uuid::new_v4().simple().to_string());
    }
    URL_SAFE_NO_PAD.encode(raw.as_bytes())
}
