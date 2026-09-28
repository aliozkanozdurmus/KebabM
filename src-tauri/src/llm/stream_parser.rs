// Sub-PRD 5: SSE parser + NDJSON parser utilities

use serde_json::Value;

/// Parses Server-Sent Events (SSE) format.
/// SSE lines look like: "data: {json...}" or "data: [DONE]"
pub struct SSEParser;

impl SSEParser {
    /// Parse a raw SSE chunk into individual data lines.
    /// Returns a list of parsed JSON values. Returns None for "[DONE]" signals.
    pub fn parse_chunk(chunk: &str) -> Vec<Option<Value>> {
        let mut results = Vec::new();

        for line in chunk.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(':') {
                // Empty line (event boundary) or comment
                continue;
            }

            if let Some(data) = line.strip_prefix("data:") {
                let data = data.trim();
                if data == "[DONE]" {
                    results.push(None);
                    continue;
                }
                if data.is_empty() {
                    continue;
                }
                match serde_json::from_str::<Value>(data) {
                    Ok(val) => results.push(Some(val)),
                    Err(_) => {
                        // Not valid JSON yet, might be a partial line - skip
                    }
                }
            }
        }

        results
    }

    /// Extract token text from an OpenAI-compatible SSE data object.
    /// Path: choices[0].delta.content
    pub fn extract_openai_token(data: &Value) -> Option<String> {
        data.get("choices")
            .and_then(|c| c.get(0))
            .and_then(|choice| choice.get("delta"))
            .and_then(|delta| delta.get("content"))
            .and_then(|content| content.as_str())
            .map(|s| s.to_string())
    }

    /// Extract token text from an Anthropic SSE content_block_delta event.
    /// Path: delta.text
    pub fn extract_anthropic_token(data: &Value) -> Option<String> {
        data.get("delta")
            .and_then(|delta| delta.get("text"))
            .and_then(|text| text.as_str())
            .map(|s| s.to_string())
    }

    /// Extract web search grounding sources from a Gemini SSE data object, if present.
    /// Path: candidates[0].groundingMetadata.groundingChunks[].web.{uri,title}
    pub fn extract_gemini_grounding(data: &Value) -> Vec<(String, String)> {
        data.get("candidates")
            .and_then(|c| c.get(0))
            .and_then(|candidate| candidate.get("groundingMetadata"))
            .and_then(|gm| gm.get("groundingChunks"))
            .and_then(|chunks| chunks.as_array())
            .map(|chunks| {
                chunks
                    .iter()
                    .filter_map(|chunk| {
                        let web = chunk.get("web")?;
                        let uri = web.get("uri")?.as_str()?.to_string();
                        let title = web
                            .get("title")
                            .and_then(|t| t.as_str())
                            .unwrap_or(&uri)
                            .to_string();
                        Some((title, uri))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Extract token text from a Gemini SSE data object.
    /// Path: candidates[0].content.parts[0].text
    pub fn extract_gemini_token(data: &Value) -> Option<String> {
        data.get("candidates")
            .and_then(|c| c.get(0))
            .and_then(|candidate| candidate.get("content"))
            .and_then(|content| content.get("parts"))
            .and_then(|parts| parts.get(0))
            .and_then(|part| part.get("text"))
            .and_then(|text| text.as_str())
            .map(|s| s.to_string())
    }
}

/// Parses Newline-Delimited JSON (NDJSON) format.
/// Each line is a separate JSON object.
pub struct NDJSONParser;

impl NDJSONParser {
    /// Parse a raw NDJSON chunk into JSON values.
    /// Each non-empty line should be a valid JSON object.
    pub fn parse_chunk(chunk: &str) -> Vec<Value> {
        let mut results = Vec::new();

        for line in chunk.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            match serde_json::from_str::<Value>(line) {
                Ok(val) => results.push(val),
                Err(_) => {
                    // Partial line or invalid JSON - skip
                }
            }
        }

        results
    }

    /// Extract token text from an Ollama NDJSON object.
    /// Path: message.content
    pub fn extract_ollama_token(data: &Value) -> Option<String> {
        data.get("message")
            .and_then(|msg| msg.get("content"))
            .and_then(|content| content.as_str())
            .map(|s| s.to_string())
    }

    /// Check if an Ollama NDJSON object indicates the stream is done.
    /// Field: done == true
    pub fn is_ollama_done(data: &Value) -> bool {
        data.get("done")
            .and_then(|d| d.as_bool())
            .unwrap_or(false)
    }
}

/// A line buffer for incremental SSE/NDJSON parsing from byte streams.
/// Accumulates bytes until complete lines are available.
pub struct LineBuffer {
    buffer: Vec<u8>,
}

impl LineBuffer {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
        }
    }

    /// Push new bytes into the buffer and extract complete lines.
    pub fn push(&mut self, data: &str) -> Vec<String> { self.push_bytes(data.as_bytes()) }

    pub fn push_bytes(&mut self, data: &[u8]) -> Vec<String> {
        self.buffer.extend_from_slice(data);
        let mut lines = Vec::new();
        while let Some(pos) = self.buffer.iter().position(|b| *b == b'\n') {
            let raw: Vec<_> = self.buffer.drain(..=pos).collect();
            lines.push(String::from_utf8_lossy(&raw[..pos]).into_owned());
        }
        lines
    }

    /// Get any remaining data in the buffer (for final flush).
    pub fn flush(&mut self) -> Option<String> {
        if self.buffer.is_empty() {
            None
        } else {
            Some(String::from_utf8_lossy(&std::mem::take(&mut self.buffer)).into_owned())
        }
    }
}

#[cfg(test)]
mod fragmentation_tests {
    use super::*;
    #[test]
    fn every_utf8_split_preserves_turkish_sse() {
        let text = "data: {\"choices\":[{\"delta\":{\"content\":\"Türkçe: ışık, öğün\"}}]}\r\n\n";
        for split in 0..text.len() {
            let mut b = LineBuffer::new();
            let mut lines = b.push_bytes(&text.as_bytes()[..split]);
            lines.extend(b.push_bytes(&text.as_bytes()[split..]));
            let token = lines.iter().flat_map(|l| SSEParser::parse_chunk(l)).flatten()
                .filter_map(|v| SSEParser::extract_openai_token(&v)).collect::<String>();
            assert_eq!(token, "Türkçe: ışık, öğün");
        }
    }
}
