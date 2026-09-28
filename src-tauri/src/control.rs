use serde_json::{json, Value};
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use uuid::Uuid;

use crate::state::AppState;

mod extended;

const PORTS: &[u16] = &[47331, 47332, 47333];
const STORE_FILE: &str = "config.json";
const MAX_BODY: usize = 4_000_000;

const SETTING_KEYS: &[&str] = &[
    "theme",
    "appearance",
    "aiReplyLanguage",
    "sttProvider",
    "sttLanguage",
    "llmProvider",
    "llmModel",
    "micDeviceId",
    "systemDeviceId",
    "recordingEnabled",
    "meetingAudioConfig",
    "autoTrigger",
    "autoSummary",
    "contextWindowSeconds",
    "startOnLogin",
    "firstRunCompleted",
    "hotkeys",
    "activeWhisperModel",
    "whisperDualPass",
    "contextStrategy",
    "deepgramConfig",
    "groqConfig",
    "pauseThresholdMs",
    "activeModelPerEngine",
    "diarizationEnabled",
    "noisePreset",
    "confidenceThreshold",
    "confidenceHighlightEnabled",
    "overlayOpacity",
    "transcriptFontSize",
    "translationFontSize",
    "transcriptTextColor",
    "translationTextColor",
    "aiResponseTextColor",
    "aiResponseFontSize",
    "showPostMeetingTranslation",
    "trayNotifications",
    "trayAutoStart",
    "trayStartMinimized",
    "trayAutoDetectMeeting",
    "trayStealthEnabled",
    "stealthShortcut",
    "rememberedMeetingSetup",
];

const SECRET_NAMES: &[&str] = &[
    "translation_microsoft",
    "translation_google",
    "translation_deepl",
    "openai",
    "anthropic",
    "gemini",
    "groq",
    "groq_whisper",
    "xai",
    "openrouter",
    "codex",
    "mistral",
    "deepseek",
    "together",
    "fireworks",
    "cerebras",
    "perplexity",
    "cohere",
    "sambanova",
    "nvidia",
    "github",
    "moonshot",
    "qwen",
    "zhipu",
    "azure",
    "azure_endpoint",
    "azure_speech",
    "azure_speech_region",
    "deepgram",
    "whisper_api",
    "chatgpt_access_token",
];

pub fn start(app: AppHandle) {
    let token = Uuid::new_v4().simple().to_string();
    let app_for_task = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut bound = None;
        for port in PORTS {
            if let Ok(listener) = TcpListener::bind(("127.0.0.1", *port)).await {
                bound = Some((listener, *port));
                break;
            }
        }
        let Some((listener, port)) = bound else {
            log::warn!("zaiqoM control server could not bind a local port");
            return;
        };
        if let Ok(dir) = app_for_task.path().app_data_dir() {
            let _ = std::fs::create_dir_all(&dir);
            let file = dir.join("zaiqo-control.json");
            let body = json!({ "port": port, "token": token, "host": "127.0.0.1" });
            if let Err(e)=write_control_file(&file,&body.to_string()){log::error!("Cannot protect control token file: {e}");return;}
            log::info!("zaiqoM control file written to {}", file.display());
        }
        log::info!("zaiqoM control server listening on 127.0.0.1:{port}");
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                continue;
            };
            let app = app_for_task.clone();
            let token = token.clone();
            tokio::spawn(async move {
                let _ = handle_client(&mut socket, &app, &token).await;
            });
        }
    });
}

async fn handle_client(
    socket: &mut tokio::net::TcpStream,
    app: &AppHandle,
    token: &str,
) -> Result<(), ()> {
    let (headers, body) = tokio::time::timeout(Duration::from_secs(30), read_http(socket)).await.map_err(|_| ())??;
    let mut parts = headers.lines().next().unwrap_or("").split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("").split('?').next().unwrap_or("");

    if method.eq_ignore_ascii_case("GET") && path == "/health" {
        return write_json(socket, 200, json!({"ok": true, "name": "zaiqoM"})).await;
    }
    if !(method.eq_ignore_ascii_case("POST") && path == "/tool") {
        return write_json(socket, 404, json!({"ok": false, "error": "Not found"})).await;
    }
    if !authorized(&headers, token) {
        return write_json(socket, 401, json!({"ok": false, "error": "Unauthorized"})).await;
    }

    let payload: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    let tool = payload.get("tool").and_then(|v| v.as_str()).unwrap_or("");
    let args = payload.get("arguments").cloned().unwrap_or(json!({}));
    log::info!("zaiqoM control tool: {tool}");
    // An MCP cancellation closes the HTTP connection. Dropping dispatch drops
    // provider futures and their request leases. Blocking file discovery may finish
    // in the background, but cannot replace the index after dispatch is dropped.
    let mut disconnected = [0u8; 1];
    let result = tokio::select! {
        result = dispatch(app, tool, args) => result,
        _ = socket.read(&mut disconnected) => return Ok(()),
        _ = tokio::time::sleep(Duration::from_secs(1800)) => json!({"ok":false,"error":"Operation timed out; inspect state before retrying"}),
    };
    let status = if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
        200
    } else {
        400
    };
    write_json(socket, status, result).await
}

async fn read_http(socket: &mut tokio::net::TcpStream) -> Result<(String, Vec<u8>), ()> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 8192];
    let header_end = loop {
        if buf.len() > MAX_BODY {
            return Err(());
        }
        let n = tokio::time::timeout(Duration::from_secs(20), socket.read(&mut tmp))
            .await
            .map_err(|_| ())?
            .map_err(|_| ())?;
        if n == 0 {
            return Err(());
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break pos;
        }
        if buf.len() > 65_536 {
            return Err(());
        }
    };

    let headers = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let mut content_length = 0usize;
    for line in headers.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("content-length") {
            content_length = value.trim().parse().unwrap_or(0);
        }
    }
    if content_length > MAX_BODY {
        return Err(());
    }
    let body_at = header_end + 4;
    while buf.len() < body_at + content_length {
        let n = tokio::time::timeout(Duration::from_secs(20), socket.read(&mut tmp))
            .await
            .map_err(|_| ())?
            .map_err(|_| ())?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if buf.len() > MAX_BODY {
            return Err(());
        }
    }
    let end = (body_at + content_length).min(buf.len());
    let body = if end > body_at {
        buf[body_at..end].to_vec()
    } else {
        Vec::new()
    };
    Ok((headers, body))
}

fn authorized(headers: &str, token: &str) -> bool {
    // Browser pages cannot call the local control API, even with a leaked token.
    if headers.lines().any(|l|l.split_once(':').is_some_and(|(n,_)|n.trim().eq_ignore_ascii_case("origin"))){return false;}
    let expected = format!("Bearer {token}");
    headers.lines().any(|line| {
        let Some((name, value)) = line.split_once(':') else {
            return false;
        };
        name.trim().eq_ignore_ascii_case("authorization") && value.trim() == expected
    })
}

async fn write_json(socket: &mut tokio::net::TcpStream, status: u16, body: Value) -> Result<(), ()> {
    let text = if status == 200 {
        "OK"
    } else if status == 401 {
        "Unauthorized"
    } else if status == 404 {
        "Not Found"
    } else {
        "Bad Request"
    };
    let payload = body.to_string();
    let response = format!(
        "HTTP/1.1 {status} {text}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
        payload.len()
    );
    socket.write_all(response.as_bytes()).await.map_err(|_| ())
}

async fn dispatch(app: &AppHandle, tool: &str, args: Value) -> Value {
    let outcome = match tool {
        "status" => status(app),
        "platform" => serde_json::to_value(crate::platform::current()).map_err(|e| e.to_string()),
        "get_settings" => get_settings(app),
        "set_settings" => set_settings(app, &args).await,
        "list_llm_providers" => {
            serde_json::to_value(crate::llm::LLMRouter::get_all_providers()).map_err(|e| e.to_string())
        }
        "configure_llm" => configure_llm(app, &args, true).await,
        "list_models" => list_models(app, &args).await,
        "test_llm" => test_llm(app, &args).await,
        "sign_in_chatgpt" => sign_in_chatgpt(app).await,
        "store_secret" => store_secret(app, &args),
        "delete_secret" => delete_secret(app, &args),
        "list_configured_secrets" => list_configured_secrets(app),
        "list_stt_providers" => list_stt_providers().await,
        "configure_stt" => configure_stt(app, &args).await,
        "list_audio_devices" => list_audio_devices().await,
        "list_projects" => list_projects(app),
        "get_project" => get_project(app, &args),
        "create_project" => create_project(app, &args),
        "set_active_project" => set_active_project(app, &args),
        "delete_project" => delete_project(app, &args),
        "scan_project" => scan_project(app, &args).await,
        "list_meetings" => list_meetings(app, &args),
        "get_meeting" => get_meeting(app, &args),
        "search_meetings" => search_meetings(app, &args),
        "import_transcript" => import_transcript(app, &args),
        "start_meeting" => extended::dispatch(app, tool, &args).await,
        "end_meeting" => extended::dispatch(app, tool, &args).await,
        "rename_meeting" => rename_meeting(app, &args).await,
        "delete_meeting" => delete_meeting(app, &args).await,
        "set_custom_instructions" => set_custom_instructions(app, &args).await,
        _ => extended::dispatch(app, tool, &args).await,
    };
    match outcome {
        Ok(result) => json!({"ok": true, "result": result}),
        Err(error) => json!({"ok": false, "error": error}),
    }
}

fn status(app: &AppHandle) -> Result<Value, String> {
    let state = app.state::<AppState>();
    let language = state
        .stt_language
        .read()
        .map(|v| v.clone())
        .unwrap_or_else(|_| "en-US".to_string());
    let db = lock_db(&state)?;
    let project = crate::projects::active(db.connection())?;
    let open_meeting = open_live_meeting(db.connection())?;
    drop(db);
    Ok(json!({
        "name": "ZaiqoM-MeetingHelper",
        "version": env!("CARGO_PKG_VERSION"),
        "language": language,
        "settings": picked_settings(app)?,
        "active_project": project,
        "open_meeting": open_meeting,
        "secrets_present": present_secrets(&state)?,
        "platform": crate::platform::current(),
        "data_dir": app.path().app_data_dir().ok().map(|p| p.display().to_string()),
    }))
}

fn open_live_meeting(conn: &rusqlite::Connection) -> Result<Option<Value>, String> {
    let mut stmt = conn
        .prepare("SELECT id, title, project_id FROM meetings WHERE end_time IS NULL AND source = 'live' ORDER BY start_time DESC LIMIT 1")
        .map_err(|e| e.to_string())?;
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    let Some(row) = rows.next().map_err(|e| e.to_string())? else {
        return Ok(None);
    };
    let id: String = row.get(0).map_err(|e| e.to_string())?;
    let title: String = row.get(1).map_err(|e| e.to_string())?;
    let project_id: Option<String> = row.get(2).map_err(|e| e.to_string())?;
    Ok(Some(json!({"id": id, "title": title, "project_id": project_id})))
}

fn get_settings(app: &AppHandle) -> Result<Value, String> {
    picked_settings(app)
}

fn picked_settings(app: &AppHandle) -> Result<Value, String> {
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    let mut out = serde_json::Map::new();
    for key in SETTING_KEYS {
        if let Some(value) = store.get(*key) {
            out.insert((*key).to_string(), value);
        }
    }
    Ok(Value::Object(out))
}

async fn set_settings(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let values = args.get("values").filter(|v| v.is_object()).unwrap_or(args);
    let Some(map) = values.as_object() else {
        return Err("settings must be an object".to_string());
    };
    if let Some(value) = map.get("appearance") {
        if !matches!(value.as_str(), Some("ibm" | "liquid-glass" | "apple" | "linear" | "notion" | "material" | "github" | "terminal")) {
            return Err("Unknown appearance. Use ibm, liquid-glass, apple, linear, notion, material, github, or terminal.".to_string());
        }
    }
    let mut unknown = Vec::new();
    for key in map.keys() {
        if !setting_allowed(key) {
            unknown.push(key.clone());
        }
    }
    if !unknown.is_empty() {
        return Err(format!(
            "These settings cannot be changed from here: {}",
            unknown.join(", ")
        ));
    }

    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    let mut keys = Vec::new();
    for (key, value) in map {
        store.set(key.clone(), value.clone());
        keys.push(key.clone());
    }
    if let Some(enabled) = map.get("recordingEnabled").and_then(|v| v.as_bool()) {
        if let Some(mut audio) = store.get("meetingAudioConfig") {
            if let Some(obj) = audio.as_object_mut() {
                obj.insert("recording_enabled".to_string(), json!(enabled));
                store.set("meetingAudioConfig", audio);
            }
        }
    }
    store.save().map_err(|e| e.to_string())?;

    let mut warnings = Vec::new();
    if let Some(language) = map.get("aiReplyLanguage").and_then(|v| v.as_str()) {
        if let Err(error) = crate::commands::intelligence_commands::set_ai_reply_language(language.to_string(), app.state()) {
            warnings.push(error);
        }
    }
    if let Some(language) = map.get("sttLanguage").and_then(|v| v.as_str()) {
        if let Err(error) = crate::commands::stt_commands::set_stt_language(app.clone(), language.to_string()).await {
            warnings.push(error);
        }
    }
    if let Some(provider) = map.get("sttProvider").and_then(|v| v.as_str()) {
        if let Err(error) = crate::commands::stt_commands::set_stt_provider(app.clone(), provider.to_string()).await {
            warnings.push(error);
        }
    }
    if let Some(config) = map.get("deepgramConfig") {
        if let Err(error) = crate::commands::stt_commands::update_deepgram_config(app.clone(), config.to_string()).await {
            warnings.push(error);
        }
    }
    if let Some(config) = map.get("groqConfig") {
        if let Err(error) = crate::commands::stt_commands::update_groq_config(app.clone(), config.to_string()).await {
            warnings.push(error);
        }
    }
    if let Some(ms) = map.get("pauseThresholdMs").and_then(as_u64) {
        if let Err(error) = crate::commands::stt_commands::set_pause_threshold(app.clone(), ms).await {
            warnings.push(error);
        }
    }
    if let Some(config) = map.get("whisperDualPass") {
        let short = num_field(config, &["shortChunkSecs", "short_chunk_secs"]).unwrap_or(1.0);
        let long = num_field(config, &["longChunkSecs", "long_chunk_secs"]).unwrap_or(3.0);
        let pause = num_field(config, &["pauseSecs", "pause_secs"]).unwrap_or(1.5);
        if let Err(error) = crate::commands::stt_commands::update_whisper_dual_pass_config(app.clone(), short, long, pause).await {
            warnings.push(error);
        }
    }
    if map.contains_key("llmProvider") || map.contains_key("llmModel") {
        if let Err(error) = activate_saved_llm(app).await {
            warnings.push(error);
        }
    }
    if let Some(enabled) = map.get("recordingEnabled").and_then(|v| v.as_bool()) {
        if let Err(error) = crate::commands::audio_commands::set_recording_enabled(app.clone(), enabled).await {
            warnings.push(error);
        }
    }

    Ok(json!({"updated": keys, "warnings": warnings}))
}

async fn activate_saved_llm(app: &AppHandle) -> Result<(), String> {
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    let Some(provider) = store.get("llmProvider").and_then(|v| v.as_str().map(|s| s.to_string())) else {
        return Ok(());
    };
    let model = store.get("llmModel").and_then(|v| v.as_str().map(|s| s.to_string()));
    drop(store);
    configure_llm(
        app,
        &json!({"provider": provider, "model": model}),
        false,
    )
    .await
    .map(|_| ())
}

async fn configure_llm(app: &AppHandle, args: &Value, persist_choice: bool) -> Result<Value, String> {
    let provider = required_string(args, "provider")?;
    let known = crate::llm::LLMRouter::get_all_providers()
        .iter()
        .any(|item| item.provider_type == provider);
    if !known {
        return Err(format!("Unknown model provider: {provider}"));
    }
    let model = optional_string(args, "model");
    let supplied_key = optional_string(args, "api_key");
    let mut base_url = optional_string(args, "base_url");
    let state = app.state::<AppState>();

    if let Some(key) = &supplied_key {
        creds(&state)?.store_key(&provider, key)?;
    }
    if provider == "azure" {
        if let Some(url) = &base_url {
            creds(&state)?.store_key("azure_endpoint", url)?;
        } else {
            base_url = creds(&state)?.get_key("azure_endpoint")?;
        }
    }

    let stored_key = if supplied_key.is_some() {
        supplied_key.clone()
    } else {
        creds(&state)?.get_key(&provider)?
    };
    let mut config = json!({"provider_type": provider});
    if let Some(key) = &stored_key {
        config["api_key"] = json!(key);
    }
    if let Some(url) = &base_url {
        config["base_url"] = json!(url);
    }
    if let Some(auth_type) = optional_string(args, "auth_type") {
        config["auth_type"] = json!(auth_type);
    }
    if let Some(auth_value) = optional_string(args, "auth_value") {
        config["auth_value"] = json!(auth_value);
    }

    let secret = stored_key.clone();
    crate::commands::llm_commands::set_llm_provider(config.to_string(), app.state())
        .await
        .map_err(|error| hide_secret(error, secret.as_deref()))?;
    if let Some(model) = &model {
        if !model.is_empty() {
            crate::commands::llm_commands::set_active_model(provider.clone(), model.clone(), app.state())
                .await?;
        }
    }

    if persist_choice {
        let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
        store.set("llmProvider", json!(provider));
        if let Some(model) = &model {
            store.set("llmModel", json!(model));
        }
        store.save().map_err(|e| e.to_string())?;
    }

    Ok(json!({
        "provider": provider,
        "model": model,
        "key_present": stored_key.is_some() || provider == "ollama" || provider == "lm_studio" || provider == "chatgpt",
    }))
}

fn inspection_config(app: &AppHandle, args: &Value) -> Result<String,String> {
    let Some(provider)=optional_string(args,"provider") else{return Ok("{}".into());};
    if !crate::llm::LLMRouter::get_all_providers().iter().any(|p|p.provider_type==provider){return Err("Unknown provider".into());}
    let state=app.state::<AppState>();
    let key=creds(&state)?.get_key(&provider)?;
    let base=if provider=="azure"{creds(&state)?.get_key("azure_endpoint")?}else{None};
    Ok(json!({"provider_type":provider,"api_key":key,"base_url":base}).to_string())
}
async fn list_models(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let raw=crate::commands::llm_commands::list_models(inspection_config(app,args)?,app.state()).await?;
    serde_json::from_str(&raw).map_err(|e|e.to_string())
}
async fn test_llm(app: &AppHandle, args: &Value) -> Result<Value,String> {
    let ok=crate::commands::llm_commands::test_llm_connection(inspection_config(app,args)?,optional_string(args,"model"),app.state()).await?;
    Ok(json!({"ok":ok}))
}

async fn sign_in_chatgpt(app: &AppHandle) -> Result<Value, String> {
    crate::commands::llm_commands::sign_in_with_chatgpt(app.clone()).await?;
    Ok(json!({"signed_in": true}))
}

fn store_secret(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let provider = secret_name(args)?;
    let key = required_string(args, "key")?;
    creds(&app.state())?.store_key(&provider, &key)?;
    Ok(json!({"provider": provider, "stored": true}))
}

fn delete_secret(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let provider = secret_name(args)?;
    creds(&app.state())?.delete_key(&provider)?;
    Ok(json!({"provider": provider, "deleted": true}))
}

fn list_configured_secrets(app: &AppHandle) -> Result<Value, String> {
    Ok(json!(present_secrets(&app.state())?))
}

fn present_secrets(state: &AppState) -> Result<Vec<String>, String> {
    let cred = creds(state)?;
    let mut present = Vec::new();
    for name in SECRET_NAMES {
        if cred.has_key(name).unwrap_or(false) {
            present.push((*name).to_string());
        }
    }
    Ok(present)
}

async fn list_stt_providers() -> Result<Value, String> {
    let raw = crate::commands::stt_commands::get_available_stt_providers().await?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

async fn configure_stt(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let mut warnings = Vec::new();
    if let Some(language) = optional_string(args, "language") {
        crate::commands::stt_commands::set_stt_language(app.clone(), language.clone()).await?;
        write_setting(app, "sttLanguage", json!(language))?;
    }
    if let Some(provider) = optional_string(args, "provider") {
        if let Err(error) = crate::commands::stt_commands::set_stt_provider(app.clone(), provider.clone()).await {
            warnings.push(error);
        }
        write_setting(app, "sttProvider", json!(provider))?;
    }
    if let Some(config) = args.get("deepgram").filter(|v| v.is_object()) {
        crate::commands::stt_commands::update_deepgram_config(app.clone(), config.to_string()).await?;
        write_setting(app, "deepgramConfig", config.clone())?;
    }
    if let Some(config) = args.get("groq").filter(|v| v.is_object()) {
        crate::commands::stt_commands::update_groq_config(app.clone(), config.to_string()).await?;
        write_setting(app, "groqConfig", config.clone())?;
    }
    if let Some(ms) = args.get("pause_ms").and_then(as_u64) {
        crate::commands::stt_commands::set_pause_threshold(app.clone(), ms).await?;
        write_setting(app, "pauseThresholdMs", json!(ms))?;
    }

    let you_provider = optional_string(args, "you_provider");
    let them_provider = optional_string(args, "them_provider");
    let you_device = optional_string(args, "you_device_id");
    let them_device = optional_string(args, "them_device_id");
    if you_provider.is_some() || them_provider.is_some() || you_device.is_some() || them_device.is_some() {
        let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
        let mut audio = store.get("meetingAudioConfig").unwrap_or_else(default_meeting_audio);
        if let Some(obj) = audio.as_object_mut() {
            patch_party(obj, "you", you_provider.as_deref(), you_device.as_deref());
            patch_party(obj, "them", them_provider.as_deref(), them_device.as_deref());
        }
        store.set("meetingAudioConfig", audio.clone());
        store.save().map_err(|e| e.to_string())?;
    }

    Ok(json!({"applied": true, "warnings": warnings}))
}

fn patch_party(audio: &mut serde_json::Map<String, Value>, party: &str, provider: Option<&str>, device: Option<&str>) {
    let entry = audio.entry(party.to_string()).or_insert_with(|| json!({}));
    if !entry.is_object() {
        *entry = json!({});
    }
    if let Some(obj) = entry.as_object_mut() {
        if let Some(provider) = provider {
            obj.insert("stt_provider".to_string(), json!(provider));
        }
        if let Some(device) = device {
            obj.insert("device_id".to_string(), json!(device));
        }
    }
}

fn default_meeting_audio() -> Value {
    json!({
        "you": {"role": "You", "device_id": "default", "is_input_device": true, "stt_provider": "web_speech"},
        "them": {"role": "Them", "device_id": "default", "is_input_device": false, "stt_provider": "deepgram"},
        "recording_enabled": false,
        "preset_name": null
    })
}

async fn list_audio_devices() -> Result<Value, String> {
    let raw = crate::commands::audio_commands::list_audio_devices().await?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

fn list_projects(app: &AppHandle) -> Result<Value, String> {
    let projects = with_db(app, |conn| crate::projects::list(conn))?;
    serde_json::to_value(projects).map_err(|e| e.to_string())
}

fn get_project(app: &AppHandle, args: &Value) -> Result<Value, String> {
    with_db(app, |conn| {
        let id = match optional_string(args, "id") {
            Some(id) => id,
            None => crate::projects::active(conn)?
                .map(|project| project.id)
                .ok_or_else(|| "No active project".to_string())?,
        };
        let project = crate::projects::get(conn, &id)?;
        let modules = crate::projects::modules(conn, &id)?;
        Ok(json!({"project": project, "modules": modules}))
    })
}

fn create_project(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let name = required_string(args, "name")?;
    let root_path = required_string(args, "root_path")?;
    let project = with_db(app, |conn| crate::projects::create(conn, &name, &root_path))?;
    serde_json::to_value(project).map_err(|e| e.to_string())
}

fn set_active_project(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let id = args.get("id").and_then(|v| v.as_str()).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    with_db(app, |conn| crate::projects::set_active(conn, id.as_deref()))?;
    Ok(json!({"id": id}))
}

fn delete_project(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let id = required_string(args, "id")?;
    with_db(app, |conn| crate::projects::delete(conn, &id))?;
    Ok(json!({"deleted": id}))
}

async fn scan_project(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let id = required_string(args, "id")?;
    let raw = crate::commands::project_commands::scan_project(app.clone(), id).await?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

fn list_meetings(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let limit = args.get("limit").and_then(as_u64).unwrap_or(50).clamp(1, 200) as u32;
    let offset = args.get("offset").and_then(as_u64).unwrap_or(0) as u32;
    let meetings = with_db(app, |conn| {
        crate::db::meetings::list_meetings(conn, limit, offset).map_err(|e| e.to_string())
    })?;
    serde_json::to_value(meetings).map_err(|e| e.to_string())
}

fn get_meeting(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let id = required_string(args, "id")?;
    let meeting = with_db(app, |conn| {
        crate::db::meetings::get_meeting(conn, &id).map_err(|e| e.to_string())
    })?;
    serde_json::to_value(meeting).map_err(|e| e.to_string())
}

fn search_meetings(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let query = required_string(args, "query")?;
    let meetings = with_db(app, |conn| {
        crate::db::meetings::search_meetings(conn, &query).map_err(|e| e.to_string())
    })?;
    serde_json::to_value(meetings).map_err(|e| e.to_string())
}

fn import_transcript(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let project_id = required_string(args, "project_id")?;
    let title = required_string(args, "title")?;
    let transcript = required_string(args, "transcript")?;
    let meeting = with_db(app, |conn| {
        crate::db::meetings::import_transcript(conn, &project_id, &title, &transcript).map_err(|e| e.to_string())
    })?;
    serde_json::to_value(meeting).map_err(|e| e.to_string())
}

async fn rename_meeting(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let id = required_string(args, "id")?;
    let title = required_string(args, "title")?;
    crate::commands::meeting_commands::rename_meeting(id.clone(), title.clone(), app.state()).await?;
    Ok(json!({"id": id, "title": title}))
}

async fn delete_meeting(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let id = required_string(args, "id")?;
    crate::commands::meeting_commands::delete_meeting(id.clone(), app.state()).await?;
    Ok(json!({"deleted": id}))
}

async fn set_custom_instructions(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let text = args
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "text is required".to_string())?
        .to_string();
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    let mut configs = store.get("ai_action_configs").unwrap_or_else(|| json!({}));
    if !configs.is_object() {
        configs = json!({});
    }
    if let Some(obj) = configs.as_object_mut() {
        obj.insert("customInstructions".to_string(), json!(text));
    }
    store.set("ai_action_configs", configs);
    store.save().map_err(|e| e.to_string())?;
    crate::commands::context_commands::set_custom_instructions(text.clone(), app.state()).await?;
    Ok(json!({"saved": true, "chars": text.chars().count()}))
}

fn write_setting(app: &AppHandle, key: &str, value: Value) -> Result<(), String> {
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    store.set(key, value);
    store.save().map_err(|e| e.to_string())
}

fn creds(
    state: &AppState,
) -> Result<std::sync::MutexGuard<'_, crate::credentials::CredentialManager>, String> {
    state
        .credentials
        .as_ref()
        .ok_or_else(|| "Credential store is not ready".to_string())?
        .lock()
        .map_err(|e| e.to_string())
}

fn lock_db(
    state: &AppState,
) -> Result<std::sync::MutexGuard<'_, crate::db::DatabaseManager>, String> {
    state
        .database
        .as_ref()
        .ok_or_else(|| "Database is not ready".to_string())?
        .lock()
        .map_err(|e| e.to_string())
}

fn with_db<T>(
    app: &AppHandle,
    f: impl FnOnce(&rusqlite::Connection) -> Result<T, String>,
) -> Result<T, String> {
    let state = app.state::<AppState>();
    let db = lock_db(&state)?;
    f(db.connection())
}

fn setting_allowed(key: &str) -> bool {
    SETTING_KEYS.contains(&key)
}

fn secret_name(args: &Value) -> Result<String, String> {
    let name = required_string(args, "provider")?;
    if name.len() > 64 || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err("provider must be a short name such as xai, deepgram, or azure_endpoint".to_string());
    }
    Ok(name)
}

fn required_string(args: &Value, key: &str) -> Result<String, String> {
    optional_string(args, key).ok_or_else(|| format!("{key} is required"))
}

fn optional_string(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
fn optional_choice(args: &Value, key: &str, allowed: &[&str]) -> Result<Option<String>, String> {
    let Some(value) = optional_string(args, key) else {
        return Ok(None);
    };
    if allowed.contains(&value.as_str()) {
        Ok(Some(value))
    } else {
        Err(format!("{key} must be one of: {}", allowed.join(", ")))
    }
}

fn as_u64(value: &Value) -> Option<u64> {
    value.as_u64().or_else(|| value.as_f64().map(|n| n as u64))
}

fn num_field(value: &Value, keys: &[&str]) -> Option<f32> {
    for key in keys {
        if let Some(number) = value.get(*key).and_then(|v| v.as_f64()) {
            return Some(number as f32);
        }
    }
    None
}

fn hide_secret(message: String, secret: Option<&str>) -> String {
    match secret {
        Some(secret) if secret.len() >= 6 && message.contains(secret) => message.replace(secret, "[redacted]"),
        _ => message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_allowlist_blocks_secrets() {
        assert!(setting_allowed("sttLanguage"));
        assert!(setting_allowed("firstRunCompleted"));
        assert!(setting_allowed("meetingAudioConfig"));
        assert!(!setting_allowed("api_key"));
        assert!(!setting_allowed("apiKey"));
        assert!(!setting_allowed("token"));
    }

    #[test]
    fn secret_text_is_removed_from_errors() {
        let message = hide_secret(
            "bad key sk-test-secret-value".to_string(),
            Some("sk-test-secret-value"),
        );
        assert_eq!(message, "bad key [redacted]");
    }

    #[test]
    fn choice_rejects_unknown_mode() {
        let args = json!({"audio_mode": "phone"});
        let error = optional_choice(&args, "audio_mode", &["online", "in_person"]).unwrap_err();
        assert!(error.contains("online"));
    }
}

fn write_control_file(path:&std::path::Path,body:&str)->std::io::Result<()> {
    use std::io::Write;
    let mut options=std::fs::OpenOptions::new();options.write(true).create(true).truncate(true);
    #[cfg(unix)] {use std::os::unix::fs::OpenOptionsExt;options.mode(0o600);}
    let mut file=options.open(path)?;
    #[cfg(unix)] {use std::os::unix::fs::PermissionsExt;file.set_permissions(std::fs::Permissions::from_mode(0o600))?;}
    file.write_all(body.as_bytes())
}
