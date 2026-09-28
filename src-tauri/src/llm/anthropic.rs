// Sub-PRD 5: Anthropic Messages API client

use futures::StreamExt;
use serde_json::json;
use std::time::Instant;

use super::provider::{
    CompletionStats, GenerationParams, LLMError, LLMMessage, LLMProvider, ModelInfo,
    StreamEndPayload, StreamTokenPayload,
};
use super::stream_parser::{LineBuffer, SSEParser};

pub struct AnthropicClient {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl AnthropicClient {
    pub fn new(api_key: &str, base_url: Option<&str>) -> Self {
        Self {
            api_key: api_key.to_string(),
            base_url: base_url.unwrap_or("https://api.anthropic.com").to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Parse Anthropic-specific SSE event types.
    /// Anthropic SSE has "event:" lines followed by "data:" lines.
    fn parse_anthropic_sse(chunk: &str) -> Vec<AnthropicEvent> {
        let mut events = Vec::new();
        let mut current_event_type: Option<String> = None;

        for line in chunk.lines() {
            let line = line.trim();
            if line.is_empty() {
                current_event_type = None;
                continue;
            }

            if let Some(event_type) = line.strip_prefix("event:") {
                current_event_type = Some(event_type.trim().to_string());
                continue;
            }

            if let Some(data) = line.strip_prefix("data:") {
                let data = data.trim();
                if data.is_empty() {
                    continue;
                }
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(data) {
                    let event_type = current_event_type
                        .clone()
                        .or_else(|| {
                            val.get("type")
                                .and_then(|t| t.as_str())
                                .map(|s| s.to_string())
                        })
                        .unwrap_or_default();

                    events.push(AnthropicEvent {
                        event_type,
                        data: val,
                    });
                }
                current_event_type = None;
            }
        }

        events
    }
}

struct AnthropicEvent {
    event_type: String,
    data: serde_json::Value,
}

#[async_trait::async_trait]
impl LLMProvider for AnthropicClient {
    fn provider_name(&self) -> &str {
        "anthropic"
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, LLMError> {
        let mut models = Vec::new();
        let mut after = String::new();
        loop {
            let mut request = self
                .client
                .get(format!("{}/v1/models", self.base_url.trim_end_matches('/')))
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .query(&[("limit", "1000")])
                .timeout(std::time::Duration::from_secs(20));
            if !after.is_empty() {
                request = request.query(&[("after_id", &after)]);
            }
            let response = request.send().await?;
            if !response.status().is_success() {
                return Err(LLMError::ProviderError(format!(
                    "Model catalog HTTP {}",
                    response.status()
                )));
            }
            let value: serde_json::Value = response.json().await?;
            let data = value["data"]
                .as_array()
                .ok_or_else(|| LLMError::InvalidResponse("Invalid model catalog".into()))?;
            for item in data {
                if let Some(id) = item["id"].as_str() {
                    models.push(ModelInfo {
                        max_output_tokens: item["max_tokens"].as_u64(),
                        capabilities: item.get("capabilities").filter(|v| !v.is_null()).cloned(),
                        id: id.into(),
                        name: item["display_name"].as_str().unwrap_or(id).into(),
                        provider: "anthropic".into(),
                        context_window: item["max_input_tokens"].as_u64(),
                    });
                }
            }
            if value["has_more"].as_bool() != Some(true) {
                break;
            }
            let next = value["last_id"]
                .as_str()
                .filter(|id| !id.is_empty() && *id != after)
                .ok_or_else(|| LLMError::InvalidResponse("Invalid model catalog cursor".into()))?;
            after = next.into();
            if models.len() > 10000 {
                return Err(LLMError::InvalidResponse("Model catalog too large".into()));
            }
        }
        Ok(models)
    }

    async fn test_connection(&self) -> Result<bool, LLMError> {
        Ok(!self.list_models().await?.is_empty())
    }

    async fn stream_completion(
        &self,
        messages: Vec<LLMMessage>,
        model: &str,
        params: GenerationParams,
        app_handle: super::request::ResponseSink,
    ) -> Result<CompletionStats, LLMError> {
        let url = format!("{}/v1/messages", self.base_url);
        let start = Instant::now();

        // Anthropic requires system message to be separate
        let mut system_content: Option<String> = None;
        let mut api_messages: Vec<serde_json::Value> = Vec::new();

        for msg in &messages {
            if msg.role == "system" {
                system_content = Some(msg.content.clone());
            } else {
                api_messages.push(json!({
                    "role": msg.role,
                    "content": msg.content
                }));
            }
        }

        let max_tokens = params.max_tokens.unwrap_or(4096);
        let mut body = json!({
            "model": model,
            "max_tokens": max_tokens,
            "messages": api_messages,
            "stream": true
        });

        // Only legacy models accept custom sampling parameters. New catalogs can
        // introduce models with different capabilities; omit optional parameters by default.
        if model.starts_with("claude-3")
            || model.starts_with("claude-haiku-4")
            || model.starts_with("claude-sonnet-4")
        {
            if let Some(temp) = params.temperature {
                body["temperature"] = json!(temp);
            }
        }
        if model.starts_with("claude-sonnet-5") && max_tokens <= 512 {
            body["thinking"] = json!({"type":"disabled"});
        }

        if let Some(system) = &system_content {
            body["system"] = json!(system);
        }

        // NOTE: llm_stream_start is emitted by IntelligenceEngine::generate_assist()
        // with the correct mode. Do NOT emit it here — it would overwrite the mode.

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                let _ = app_handle.emit("llm_stream_error", e.to_string());
                e
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            let err_msg = format!("Anthropic request failed ({}): {}", status, body);
            let _ = app_handle.emit("llm_stream_error", &err_msg);

            if status.as_u16() == 401 || status.as_u16() == 403 {
                return Err(LLMError::AuthError(err_msg));
            }
            return Err(LLMError::ProviderError(err_msg));
        }

        let mut stream = response.bytes_stream();
        let mut line_buffer = LineBuffer::new();
        let mut token_count: u64 = 0;
        let mut input_tokens: u64 = 0;
        let mut output_tokens: u64 = 0;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| {
                let _ = app_handle.emit("llm_stream_error", e.to_string());
                LLMError::HttpError(e)
            })?;

            let lines = line_buffer.push_bytes(&chunk);

            for line in lines {
                let events = Self::parse_anthropic_sse(&line);
                for event in events {
                    match event.event_type.as_str() {
                        "message_start" => {
                            // Extract input token count from usage
                            if let Some(usage) =
                                event.data.get("message").and_then(|m| m.get("usage"))
                            {
                                if let Some(it) = usage.get("input_tokens").and_then(|v| v.as_u64())
                                {
                                    input_tokens = it;
                                }
                            }
                        }
                        "content_block_delta" => {
                            if let Some(token) = SSEParser::extract_anthropic_token(&event.data) {
                                if !token.is_empty() {
                                    token_count += 1;
                                    let _ = app_handle.emit(
                                        "llm_stream_token",
                                        StreamTokenPayload {
                                            token: token.clone(),
                                        },
                                    );
                                }
                            }
                        }
                        "message_delta" => {
                            // Extract output token count
                            if let Some(usage) = event.data.get("usage") {
                                if let Some(ot) =
                                    usage.get("output_tokens").and_then(|v| v.as_u64())
                                {
                                    output_tokens = ot;
                                }
                            }
                        }
                        "message_stop" | "content_block_start" | "content_block_stop" | "ping" => {
                            // No action needed for these event types
                        }
                        "error" => {
                            let err_msg = event
                                .data
                                .get("error")
                                .and_then(|e| e.get("message"))
                                .and_then(|m| m.as_str())
                                .unwrap_or("Unknown Anthropic error")
                                .to_string();
                            let _ = app_handle.emit("llm_stream_error", &err_msg);
                            return Err(LLMError::ProviderError(err_msg));
                        }
                        _ => {}
                    }
                }
            }
        }

        // Flush remaining buffer
        if let Some(remaining) = line_buffer.flush() {
            let events = Self::parse_anthropic_sse(&remaining);
            for event in events {
                if event.event_type == "content_block_delta" {
                    if let Some(token) = SSEParser::extract_anthropic_token(&event.data) {
                        if !token.is_empty() {
                            token_count += 1;
                            let _ = app_handle.emit(
                                "llm_stream_token",
                                StreamTokenPayload {
                                    token: token.clone(),
                                },
                            );
                        }
                    }
                }
            }
        }

        let latency_ms = start.elapsed().as_millis() as u64;

        // Use API-reported counts if available
        if output_tokens == 0 {
            output_tokens = token_count;
        }
        let total = input_tokens + output_tokens;

        let stats = CompletionStats {
            prompt_tokens: input_tokens,
            completion_tokens: output_tokens,
            total_tokens: total,
            latency_ms,
        };

        let _ = app_handle.emit(
            "llm_stream_end",
            StreamEndPayload {
                total_tokens: total,
                latency_ms,
            },
        );

        Ok(stats)
    }
}
