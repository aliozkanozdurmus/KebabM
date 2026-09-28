//! Request-owned output. Background work never subscribes to application-wide tokens.
use super::provider::{CompletionStats, GenerationParams, LLMError, LLMMessage, LLMProvider};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::Emitter;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestIdentity {
    pub session_id: String,
    pub request_id: String,
    pub question_id: Option<String>,
}

#[derive(Clone)]
pub struct ResponseSink {
    app: Option<tauri::AppHandle>,
    pub identity: RequestIdentity,
    pub cancel: CancellationToken,
    text: Arc<Mutex<String>>,
}

impl ResponseSink {
    pub fn new(
        app: Option<tauri::AppHandle>,
        identity: RequestIdentity,
        cancel: CancellationToken,
    ) -> Self {
        Self {
            app,
            identity,
            cancel,
            text: Arc::new(Mutex::new(String::new())),
        }
    }

    pub fn private() -> Self {
        Self::new(
            None,
            RequestIdentity {
                session_id: "background".into(),
                request_id: uuid::Uuid::new_v4().to_string(),
                question_id: None,
            },
            CancellationToken::new(),
        )
    }

    pub fn text(&self) -> String {
        self.text
            .lock()
            .map(|v| visible_answer(&v))
            .unwrap_or_default()
    }

    /// Compatibility boundary for provider adapters. Terminal events belong to the caller.
    pub fn emit<T: Serialize>(&self, event: &str, payload: T) -> Result<(), String> {
        if self.cancel.is_cancelled() || event == "llm_stream_end" || event == "llm_stream_error" {
            return Ok(());
        }
        let mut value = serde_json::to_value(payload).map_err(|e| e.to_string())?;
        if event == "llm_stream_token" {
            if let Some(token) = value.get("token").and_then(|v| v.as_str()) {
                self.text.lock().map_err(|e| e.to_string())?.push_str(token);
            }
        }
        if let Some(object) = value.as_object_mut() {
            object.insert("requestId".into(), json!(self.identity.request_id));
            object.insert("sessionId".into(), json!(self.identity.session_id));
            object.insert("questionId".into(), json!(self.identity.question_id));
        }
        if let Some(app) = &self.app {
            app.emit(event, value).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn phase(&self, status: &str, error: Option<&str>) {
        if let Some(app) = &self.app {
            let _ = app.emit(
                "assist_event",
                json!({
                    "sessionId":self.identity.session_id,"requestId":self.identity.request_id,
                    "questionId":self.identity.question_id,"status":status,"error":error,
                }),
            );
        }
    }

    pub fn finish_assist(&self, result: &Result<CompletionStats, String>) {
        let Some(app) = &self.app else { return };
        let (event, mut payload, status) = match result {
            Ok(stats) => ("llm_stream_end", json!(stats), "completed"),
            Err(error) => (
                "llm_stream_error",
                json!({"message":error,"cancelled":error=="Answer cancelled."}),
                if error == "Answer cancelled." {
                    "cancelled"
                } else {
                    "error"
                },
            ),
        };
        payload["requestId"] = json!(self.identity.request_id);
        payload["sessionId"] = json!(self.identity.session_id);
        payload["questionId"] = json!(self.identity.question_id);
        let _ = app.emit(event, payload);
        self.phase(status, result.as_ref().err().map(String::as_str));
    }

    pub fn terminal(&self, result: &Result<CompletionStats, LLMError>) {
        let Some(app) = &self.app else { return };
        let (event, mut payload) = match result {
            Ok(stats) => ("llm_stream_end", json!(stats)),
            Err(e) => (
                "llm_stream_error",
                json!({
                    "message": user_error(e), "cancelled": matches!(e, LLMError::Cancelled),
                }),
            ),
        };
        payload["requestId"] = json!(self.identity.request_id);
        payload["sessionId"] = json!(self.identity.session_id);
        payload["questionId"] = json!(self.identity.question_id);
        let _ = app.emit(event, payload);
    }
}

/// Match the live renderer: hidden or unfinished reasoning is not an answer.
fn visible_answer(raw: &str) -> String {
    let mut remaining = raw;
    let mut answer = String::new();
    while let Some(start) = remaining.find("<think>") {
        answer.push_str(&remaining[..start]);
        let reasoning = &remaining[start + "<think>".len()..];
        match reasoning.find("</think>") {
            Some(end) => remaining = reasoning[end + "</think>".len()..].trim_start(),
            None => return answer.trim_start().to_owned(),
        }
    }
    answer.push_str(remaining);
    answer.trim_start().to_owned()
}

pub async fn complete(
    provider: &dyn LLMProvider,
    messages: Vec<LLMMessage>,
    model: &str,
    params: GenerationParams,
    sink: ResponseSink,
) -> Result<CompletionStats, LLMError> {
    let result = complete_unfinished(provider, messages, model, params, sink.clone()).await;
    sink.terminal(&result);
    result
}

/// Live assistance defers the terminal event until the answer is durably saved.
pub async fn complete_unfinished(
    provider: &dyn LLMProvider,
    messages: Vec<LLMMessage>,
    model: &str,
    params: GenerationParams,
    sink: ResponseSink,
) -> Result<CompletionStats, LLMError> {
    let result = tokio::select! {
        biased;
        _ = sink.cancel.cancelled() => Err(LLMError::Cancelled),
        result = tokio::time::timeout(Duration::from_secs(120),
            provider.stream_completion(messages, model, params, sink.clone())) => {
            result.unwrap_or_else(|_| Err(LLMError::ConnectionFailed("Request timed out. Try again.".into())))
        }
    };
    let result = result.and_then(|stats| {
        if sink.text().trim().is_empty() {
            Err(LLMError::InvalidResponse(
                "The model returned an empty answer. Try again.".into(),
            ))
        } else {
            Ok(stats)
        }
    });
    result
}

pub fn user_error(error: &LLMError) -> String {
    match error {
        LLMError::Cancelled => "Answer cancelled.".into(),
        LLMError::AuthError(_) => {
            "Authentication failed. Check the selected provider's API key.".into()
        }
        LLMError::HttpError(e) if e.is_timeout() => "The provider timed out. Try again.".into(),
        LLMError::HttpError(_) => {
            "The provider could not be reached. Check your connection and try again.".into()
        }
        _ => {
            // Provider bodies may echo prompts. Never forward them to the technical log.
            let text = error.to_string();
            if text.contains("429") {
                "Provider quota or rate limit reached. Wait and retry.".into()
            } else if text.contains("401") || text.contains("403") {
                "Provider access denied. Check credentials and model access.".into()
            } else if text.contains("404") {
                "The selected model is unavailable. Refresh the model list.".into()
            } else if matches!(error, LLMError::ProviderError(_)) {
                "The provider rejected the request. Check the selected model and retry.".into()
            } else if matches!(error, LLMError::NotConfigured(_)) {
                "Select a provider and model before requesting an answer.".into()
            } else if matches!(error, LLMError::ConnectionFailed(_)) {
                "The provider connection failed or timed out. Try again.".into()
            } else {
                "The provider returned an empty or invalid answer. Try again.".into()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persisted_answer_matches_visible_stream_content() {
        let sink = ResponseSink::private();
        for token in [
            "<thi",
            "nk>private",
            "</think>\nTürkçe cevap.",
            "<think>unfinished",
        ] {
            sink.emit("llm_stream_token", json!({"token":token}))
                .unwrap();
        }
        assert_eq!(sink.text(), "Türkçe cevap.");
        assert_eq!(visible_answer("<think>private</think>\n"), "");
        assert_eq!(visible_answer("<think>unfinished"), "");
    }
    #[test]
    fn background_tokens_are_isolated_and_cancelled_tokens_ignored() {
        let a = ResponseSink::private();
        let b = ResponseSink::private();
        a.emit("llm_stream_token", json!({"token":"one"})).unwrap();
        b.emit("llm_stream_token", json!({"token":"two"})).unwrap();
        a.cancel.cancel();
        a.emit("llm_stream_token", json!({"token":"late"})).unwrap();
        assert_eq!(a.text(), "one");
        assert_eq!(b.text(), "two");
    }
}

#[cfg(test)]
mod network_tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    async fn server(status: u16, body: String) -> (String, tokio::task::JoinHandle<String>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = vec![0; 16384];
            let count = socket.read(&mut buffer).await.unwrap();
            let request = String::from_utf8_lossy(&buffer[..count]).into_owned();
            let header=format!("HTTP/1.1 {status} Test\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len());
            socket.write_all(header.as_bytes()).await.unwrap();
            // Force tiny writes, including splits inside Turkish Unicode characters.
            for bytes in body.as_bytes().chunks(2) {
                socket.write_all(bytes).await.unwrap();
                tokio::task::yield_now().await;
            }
            request
        });
        (url, task)
    }
    #[tokio::test]
    async fn http_failures_are_sanitized_and_next_request_can_succeed() {
        for status in [401, 429, 500] {
            let (url, task) = server(status, "private-prompt-and-key".into()).await;
            let provider = crate::llm::anthropic::AnthropicClient::new("test", Some(&url));
            let error = complete(
                &provider,
                vec![],
                "claude-sonnet-5",
                GenerationParams::default(),
                ResponseSink::private(),
            )
            .await
            .unwrap_err();
            assert!(!user_error(&error).contains("private-prompt"));
            task.await.unwrap();
        }
        let body="data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"Türkçe cevap.\"}}\n\ndata: {\"type\":\"message_stop\"}\n\n";
        let (url, task) = server(200, body.into()).await;
        let sink = ResponseSink::private();
        let provider = crate::llm::anthropic::AnthropicClient::new("test", Some(&url));
        complete(
            &provider,
            vec![],
            "claude-sonnet-5",
            GenerationParams {
                temperature: Some(0.3),
                ..Default::default()
            },
            sink.clone(),
        )
        .await
        .unwrap();
        assert_eq!(sink.text(), "Türkçe cevap.");
        assert!(!task.await.unwrap().contains("temperature"));
    }
    #[tokio::test]
    async fn anthropic_catalog_keeps_capabilities_and_token_limits() {
        let (url, task)=server(200,json!({"data":[{"id":"claude-sonnet-5","display_name":"Sonnet 5","max_input_tokens":200000,"max_tokens":64000,"capabilities":{"thinking":{"supported":true}}}],"has_more":false}).to_string()).await;
        let provider = crate::llm::anthropic::AnthropicClient::new("test", Some(&url));
        let models = provider.list_models().await.unwrap();
        assert_eq!(models[0].context_window, Some(200000));
        assert_eq!(models[0].max_output_tokens, Some(64000));
        assert_eq!(
            models[0].capabilities.as_ref().unwrap()["thinking"]["supported"],
            true
        );
        assert!(task.await.unwrap().starts_with("GET /v1/models"));
    }

    #[tokio::test]
    async fn empty_response_is_an_error() {
        let (url, task) = server(200, "data: {\"type\":\"message_stop\"}\n\n".into()).await;
        let provider = crate::llm::anthropic::AnthropicClient::new("test", Some(&url));
        assert!(matches!(
            complete(
                &provider,
                vec![],
                "model",
                GenerationParams::default(),
                ResponseSink::private()
            )
            .await,
            Err(LLMError::InvalidResponse(_))
        ));
        task.await.unwrap();
    }
    #[tokio::test]
    async fn hidden_only_response_is_not_a_completed_answer() {
        let body = "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"<think>internal</think>\\n\"}}\n\ndata: {\"type\":\"message_stop\"}\n\n";
        let (url, task) = server(200, body.into()).await;
        let provider = crate::llm::anthropic::AnthropicClient::new("test", Some(&url));
        assert!(matches!(
            complete(
                &provider,
                vec![],
                "model",
                GenerationParams::default(),
                ResponseSink::private()
            )
            .await,
            Err(LLMError::InvalidResponse(_))
        ));
        task.await.unwrap();
    }
    struct PendingProvider(Arc<std::sync::atomic::AtomicBool>);
    struct DropSignal(Arc<std::sync::atomic::AtomicBool>);
    impl Drop for DropSignal {
        fn drop(&mut self) {
            self.0.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }
    #[async_trait::async_trait]
    impl LLMProvider for PendingProvider {
        fn provider_name(&self) -> &str {
            "test"
        }
        async fn list_models(&self) -> Result<Vec<super::super::provider::ModelInfo>, LLMError> {
            Ok(vec![])
        }
        async fn test_connection(&self) -> Result<bool, LLMError> {
            Ok(true)
        }
        async fn stream_completion(
            &self,
            _: Vec<LLMMessage>,
            _: &str,
            _: GenerationParams,
            _: ResponseSink,
        ) -> Result<CompletionStats, LLMError> {
            let _guard = DropSignal(self.0.clone());
            std::future::pending().await
        }
    }
    #[tokio::test]
    async fn cancellation_drops_the_active_provider_future() {
        let dropped = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let provider = PendingProvider(dropped.clone());
        let sink = ResponseSink::private();
        let cancel = sink.cancel.clone();
        let task = tokio::spawn(async move {
            complete(
                &provider,
                vec![],
                "model",
                GenerationParams::default(),
                sink,
            )
            .await
        });
        tokio::task::yield_now().await;
        cancel.cancel();
        assert!(matches!(task.await.unwrap(), Err(LLMError::Cancelled)));
        assert!(dropped.load(std::sync::atomic::Ordering::SeqCst));
    }
}
