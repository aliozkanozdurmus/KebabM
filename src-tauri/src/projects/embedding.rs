//! Embedding adapters for project evidence. Model + dimensions form the index identity.
use crate::state::AppState;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddingConfig {
    pub provider: String,
    pub model: String,
    pub dimensions: usize,
    pub base_url: String,
}
impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: "lexical".into(),
            model: "gemini-embedding-2".into(),
            dimensions: 1536,
            base_url: "http://localhost:11434".into(),
        }
    }
}
impl EmbeddingConfig {
    pub fn key(&self) -> String {
        format!("{}:{}:{}", self.provider, self.model, self.dimensions)
    }
    pub fn validate(&self) -> Result<(), String> {
        if !["lexical", "gemini", "ollama"].contains(&self.provider.as_str()) {
            return Err("Unknown embedding provider".into());
        }
        if self.dimensions == 0 || self.dimensions > 4096 {
            return Err("Embedding dimensions must be between 1 and 4096".into());
        }
        if self.provider == "gemini" && self.model != "gemini-embedding-2" {
            return Err("Use gemini-embedding-2 for this adapter".into());
        }
        if self.provider == "ollama" {
            let url = url::Url::parse(&self.base_url).map_err(|_| "Invalid Ollama URL")?;
            if !["http", "https"].contains(&url.scheme())
                || !url.username().is_empty()
                || url.password().is_some()
            {
                return Err("Use an HTTP(S) Ollama endpoint without credentials in its URL".into());
            }
        }
        Ok(())
    }
}
pub fn config(conn: &Connection, project: &str) -> Result<EmbeddingConfig, String> {
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM app_state WHERE key=?1",
            [format!("project_embedding:{project}")],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    value
        .map(|v| serde_json::from_str(&v).map_err(|e| e.to_string()))
        .unwrap_or_else(|| Ok(EmbeddingConfig::default()))
}
pub fn api_key(state: &AppState) -> Option<String> {
    state
        .credentials
        .as_ref()
        .and_then(|c| c.lock().ok())
        .and_then(|c| c.get_key("gemini").ok().flatten())
}
pub async fn embed(
    config: &EmbeddingConfig,
    key: Option<&str>,
    texts: Vec<String>,
    query: bool,
) -> Result<Vec<Vec<f32>>, String> {
    config.validate()?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|_| "Cannot initialize embedding client")?;
    let vectors: Vec<Vec<f32>> = match config.provider.as_str() {
        "gemini" => {
            let key = key
                .filter(|k| !k.is_empty())
                .ok_or("Add a Gemini API key in AI provider settings")?;
            let requests:Vec<_> = texts.into_iter().map(|t|json!({
                "model":"models/gemini-embedding-2", "outputDimensionality":config.dimensions,
                "content":{"parts":[{"text":if query {format!("task: code retrieval | query: {t}")} else {t}}]}
            })).collect();
            let response = client.post("https://generativelanguage.googleapis.com/v1beta/models/gemini-embedding-2:batchEmbedContents")
                .header("x-goog-api-key",key).json(&json!({"requests":requests})).send().await.map_err(|_|"Embedding connection failed")?;
            let status = response.status();
            if !status.is_success() {
                return Err(format!(
                    "Embedding provider returned HTTP {}; keyword search remains available",
                    status.as_u16()
                ));
            }
            let value: serde_json::Value = response
                .json()
                .await
                .map_err(|_| "Invalid embedding response")?;
            serde_json::from_value(value["embeddings"].clone())
                .map(|items: Vec<Values>| items.into_iter().map(|v| v.values).collect())
                .map_err(|_| "Missing embeddings")?
        }
        "ollama" => {
            let response = client
                .post(format!(
                    "{}/api/embed",
                    config.base_url.trim_end_matches('/')
                ))
                .json(&json!({"model":config.model,"input":texts,"truncate":false}))
                .send()
                .await
                .map_err(|_| "Ollama embedding connection failed")?;
            if !response.status().is_success() {
                return Err(format!(
                    "Ollama embedding HTTP {}; source text was not truncated",
                    response.status().as_u16()
                ));
            }
            let value: serde_json::Value = response
                .json()
                .await
                .map_err(|_| "Invalid Ollama response")?;
            serde_json::from_value(value["embeddings"].clone())
                .map_err(|_| "Missing Ollama embeddings")?
        }
        _ => return Err("Keyword-only search selected".into()),
    };
    for vector in &vectors {
        validate_vector(vector, config.dimensions)?;
    }
    Ok(vectors)
}
#[derive(Deserialize)]
struct Values {
    values: Vec<f32>,
}
fn validate_vector(vector: &[f32], dimensions: usize) -> Result<(), String> {
    if vector.len() != dimensions
        || vector.iter().any(|v| !v.is_finite())
        || vector.iter().all(|v| *v == 0.0)
    {
        return Err("Invalid embedding dimensions or values; previous vectors retained".into());
    }
    Ok(())
}
pub fn save_config(
    conn: &Connection,
    project: &str,
    config: &EmbeddingConfig,
) -> Result<(), String> {
    config.validate()?;
    conn.execute(
        "INSERT OR REPLACE INTO app_state(key,value) VALUES (?1,?2)",
        params![
            format!("project_embedding:{project}"),
            serde_json::to_string(config).map_err(|e| e.to_string())?
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
pub fn cosine(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() {
        return 0.0;
    }
    let dot: f64 = a.iter().zip(b).map(|(a, b)| *a as f64 * *b as f64).sum();
    let norm = (a.iter().map(|v| (*v as f64).powi(2)).sum::<f64>()
        * b.iter().map(|v| (*v as f64).powi(2)).sum::<f64>())
    .sqrt();
    if norm > 0.0 {
        dot / norm
    } else {
        0.0
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_malformed_vectors_and_model_changes() {
        assert!(validate_vector(&[1.0, f32::NAN], 2).is_err());
        assert!(validate_vector(&[0.0, 0.0], 2).is_err());
        assert!(validate_vector(&[1.0], 2).is_err());
        let mut a = EmbeddingConfig::default();
        let key = a.key();
        a.dimensions = 768;
        assert_ne!(key, a.key());
    }
}
