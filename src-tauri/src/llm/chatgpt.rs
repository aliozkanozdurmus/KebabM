use futures::StreamExt;
use tauri::Emitter;
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::chatgpt_auth::{self, ChatGptSession, CODEX_API};
use super::provider::{
    CompletionStats, GenerationParams, LLMError, LLMMessage, LLMProvider, ModelInfo,
    StreamEndPayload, StreamTokenPayload,
};
use super::stream_parser::{LineBuffer, SSEParser};
use crate::credentials::CredentialManager;

pub struct ChatGptClient {
    session: Mutex<ChatGptSession>,
    credentials: Option<Arc<Mutex<CredentialManager>>>,
    client: reqwest::Client,
}

impl ChatGptClient {
    pub fn new(
        session: ChatGptSession,
        credentials: Option<Arc<Mutex<CredentialManager>>>,
    ) -> Self {
        Self {
            session: Mutex::new(session),
            credentials,
            client: reqwest::Client::new(),
        }
    }

    fn current(&self) -> ChatGptSession {
        self.session
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    fn apply_auth(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let session = self.current();
        let builder = builder
            .header("Authorization", format!("Bearer {}", session.access_token))
            .header("originator", "nexq");
        if session.account_id.is_empty() {
            builder
        } else {
            builder.header("chatgpt-account-id", session.account_id)
        }
    }

    async fn refresh(&self) -> Result<(), LLMError> {
        let current = self.current();
        let next = chatgpt_auth::refresh_session(&current)
            .await
            .map_err(LLMError::AuthError)?;
        if let Some(credentials) = &self.credentials {
            if let Ok(guard) = credentials.lock() {
                let _ = chatgpt_auth::store_session(&guard, &next);
            }
        }
        *self.session.lock().unwrap_or_else(|p| p.into_inner()) = next;
        Ok(())
    }
}

fn static_models() -> Vec<ModelInfo> {
    ["gpt-5.5", "gpt-5.4", "gpt-5.4-mini", "gpt-5.3-codex-spark"]
        .into_iter()
        .map(|id| ModelInfo {
            id: id.to_string(),
            name: id.to_string(),
            provider: "chatgpt".to_string(),
            context_window: Some(400_000),
        })
        .collect()
}

fn response_delta(data: &serde_json::Value) -> Option<String> {
    let kind = data.get("type").and_then(|v| v.as_str()).unwrap_or("");
    if kind == "response.output_text.delta" {
        return data.get("delta").and_then(|v| v.as_str()).map(|s| s.to_string());
    }
    data.get("delta")
        .and_then(|d| d.get("text"))
        .or_else(|| data.get("output_text"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

#[async_trait::async_trait]
impl LLMProvider for ChatGptClient {
    fn provider_name(&self) -> &str {
        "chatgpt"
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, LLMError> {
        Ok(static_models())
    }

    async fn test_connection(&self) -> Result<bool, LLMError> {
        let response = self
            .apply_auth(self.client.post(format!("{CODEX_API}/responses")))
            .json(&json!({
                "model": "gpt-5.5",
                "input": "Reply with ok",
                "max_output_tokens": 16
            }))
            .send()
            .await?;
        if response.status().is_success() {
            return Ok(true);
        }
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(LLMError::ProviderError(format!(
            "ChatGPT subscription request failed ({status}): {body}"
        )))
    }

    async fn stream_completion(
        &self,
        messages: Vec<LLMMessage>,
        model: &str,
        _params: GenerationParams,
        app_handle: tauri::AppHandle,
    ) -> Result<CompletionStats, LLMError> {
        let start = Instant::now();
        let input: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| json!({ "role": m.role, "content": m.content }))
            .collect();
        let model = if model.is_empty() { "gpt-5.5" } else { model };
        let body = json!({
            "model": model,
            "input": input,
            "stream": true
        });

        let mut response = self
            .apply_auth(self.client.post(format!("{CODEX_API}/responses")))
            .json(&body)
            .send()
            .await?;

        if response.status().as_u16() == 401 {
            self.refresh().await?;
            response = self
                .apply_auth(self.client.post(format!("{CODEX_API}/responses")))
                .json(&body)
                .send()
                .await?;
        }

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            let err = format!("ChatGPT subscription request failed ({status}): {text}");
            let _ = app_handle.emit("llm_stream_error", &err);
            return Err(LLMError::ProviderError(err));
        }

        let mut stream = response.bytes_stream();
        let mut line_buffer = LineBuffer::new();
        let mut token_count = 0u64;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(LLMError::HttpError)?;
            let text = String::from_utf8_lossy(&chunk);
            for line in line_buffer.push(&text) {
                for event in SSEParser::parse_chunk(&line) {
                    if let Some(data) = event {
                        if let Some(token) = response_delta(&data) {
                            if !token.is_empty() {
                                token_count += 1;
                                let _ = app_handle.emit(
                                    "llm_stream_token",
                                    StreamTokenPayload { token },
                                );
                            }
                        }
                    }
                }
            }
        }

        let latency_ms = start.elapsed().as_millis() as u64;
        let _ = app_handle.emit(
            "llm_stream_end",
            StreamEndPayload {
                total_tokens: token_count,
                latency_ms,
            },
        );
        Ok(CompletionStats {
            prompt_tokens: 0,
            completion_tokens: token_count,
            total_tokens: token_count,
            latency_ms,
        })
    }
}
