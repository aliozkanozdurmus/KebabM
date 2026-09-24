pub mod anthropic;
pub mod chatgpt;
pub mod chatgpt_auth;
pub mod custom;
pub mod gemini;
pub mod gemini_cache;
pub mod ollama;
pub mod openai_compat;
pub mod openrouter_models;
pub mod provider;
pub mod stream_parser;

use provider::{LLMError, LLMProvider};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

/// Provider type enum matching the frontend LLMProviderType
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    Ollama,
    LmStudio,
    Openai,
    Anthropic,
    Groq,
    Gemini,
    Openrouter,
    /// xAI Grok. OpenAI-compatible chat completions at api.x.ai.
    Xai,
    /// OpenAI Codex models (gpt-5.5 and the Codex line) on the OpenAI API.
    Codex,
    Mistral,
    Deepseek,
    Together,
    Fireworks,
    Cerebras,
    Perplexity,
    Cohere,
    Sambanova,
    Nvidia,
    Github,
    Moonshot,
    Qwen,
    Zhipu,
    /// ChatGPT Plus, Pro, Team, or Enterprise subscription via the Codex login.
    Chatgpt,
    /// Azure OpenAI resource. Uses the api-key header and the resource URL.
    Azure,
    Custom,
}

impl ProviderType {
    pub fn from_str(s: &str) -> Result<Self, LLMError> {
        match s {
            "ollama" => Ok(Self::Ollama),
            "lm_studio" => Ok(Self::LmStudio),
            "openai" => Ok(Self::Openai),
            "anthropic" => Ok(Self::Anthropic),
            "groq" => Ok(Self::Groq),
            "gemini" => Ok(Self::Gemini),
            "openrouter" => Ok(Self::Openrouter),
            "xai" => Ok(Self::Xai),
            "codex" => Ok(Self::Codex),
            "mistral" => Ok(Self::Mistral),
            "deepseek" => Ok(Self::Deepseek),
            "together" => Ok(Self::Together),
            "fireworks" => Ok(Self::Fireworks),
            "cerebras" => Ok(Self::Cerebras),
            "perplexity" => Ok(Self::Perplexity),
            "cohere" => Ok(Self::Cohere),
            "sambanova" => Ok(Self::Sambanova),
            "nvidia" => Ok(Self::Nvidia),
            "github" => Ok(Self::Github),
            "moonshot" => Ok(Self::Moonshot),
            "qwen" => Ok(Self::Qwen),
            "zhipu" => Ok(Self::Zhipu),
            "chatgpt" => Ok(Self::Chatgpt),
            "azure" => Ok(Self::Azure),
            "custom" => Ok(Self::Custom),
            _ => Err(LLMError::NotConfigured(format!(
                "Unknown provider type: {}",
                s
            ))),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Ollama => "ollama",
            Self::LmStudio => "lm_studio",
            Self::Openai => "openai",
            Self::Anthropic => "anthropic",
            Self::Groq => "groq",
            Self::Gemini => "gemini",
            Self::Openrouter => "openrouter",
            Self::Xai => "xai",
            Self::Codex => "codex",
            Self::Mistral => "mistral",
            Self::Deepseek => "deepseek",
            Self::Together => "together",
            Self::Fireworks => "fireworks",
            Self::Cerebras => "cerebras",
            Self::Perplexity => "perplexity",
            Self::Cohere => "cohere",
            Self::Sambanova => "sambanova",
            Self::Nvidia => "nvidia",
            Self::Github => "github",
            Self::Moonshot => "moonshot",
            Self::Qwen => "qwen",
            Self::Zhipu => "zhipu",
            Self::Chatgpt => "chatgpt",
            Self::Azure => "azure",
            Self::Custom => "custom",
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Self::Ollama => "Ollama",
            Self::LmStudio => "LM Studio",
            Self::Openai => "OpenAI",
            Self::Anthropic => "Anthropic",
            Self::Groq => "Groq",
            Self::Gemini => "Google Gemini",
            Self::Openrouter => "OpenRouter",
            Self::Xai => "Grok (xAI)",
            Self::Codex => "OpenAI Codex",
            Self::Mistral => "Mistral",
            Self::Deepseek => "DeepSeek",
            Self::Together => "Together",
            Self::Fireworks => "Fireworks",
            Self::Cerebras => "Cerebras",
            Self::Perplexity => "Perplexity",
            Self::Cohere => "Cohere",
            Self::Sambanova => "SambaNova",
            Self::Nvidia => "NVIDIA",
            Self::Github => "GitHub Models",
            Self::Moonshot => "Moonshot (Kimi)",
            Self::Qwen => "Qwen",
            Self::Zhipu => "Zhipu GLM",
            Self::Chatgpt => "ChatGPT",
            Self::Azure => "Azure OpenAI",
            Self::Custom => "Custom",
        }
    }

    pub fn default_base_url(&self) -> &str {
        match self {
            Self::Ollama => "http://localhost:11434",
            Self::LmStudio => "http://localhost:1234/v1",
            Self::Openai => "https://api.openai.com/v1",
            Self::Anthropic => "https://api.anthropic.com",
            Self::Groq => "https://api.groq.com/openai/v1",
            Self::Gemini => "https://generativelanguage.googleapis.com",
            Self::Openrouter => "https://openrouter.ai/api/v1",
            Self::Xai => "https://api.x.ai/v1",
            Self::Codex => "https://api.openai.com/v1",
            Self::Mistral => "https://api.mistral.ai/v1",
            Self::Deepseek => "https://api.deepseek.com/v1",
            Self::Together => "https://api.together.xyz/v1",
            Self::Fireworks => "https://api.fireworks.ai/inference/v1",
            Self::Cerebras => "https://api.cerebras.ai/v1",
            Self::Perplexity => "https://api.perplexity.ai",
            Self::Cohere => "https://api.cohere.ai/compatibility/v1",
            Self::Sambanova => "https://api.sambanova.ai/v1",
            Self::Nvidia => "https://integrate.api.nvidia.com/v1",
            Self::Github => "https://models.inference.ai.azure.com",
            Self::Moonshot => "https://api.moonshot.ai/v1",
            Self::Qwen => "https://dashscope-intl.aliyuncs.com/compatible-mode/v1",
            Self::Zhipu => "https://open.bigmodel.cn/api/paas/v4",
            Self::Chatgpt => "https://chatgpt.com/backend-api/codex",
            Self::Azure => "https://YOUR_RESOURCE.openai.azure.com/openai/v1",
            Self::Custom => "",
        }
    }

    pub fn requires_api_key(&self) -> bool {
        matches!(
            self,
            Self::Openai
                | Self::Anthropic
                | Self::Groq
                | Self::Gemini
                | Self::Openrouter
                | Self::Xai
                | Self::Codex
                | Self::Mistral
                | Self::Deepseek
                | Self::Together
                | Self::Fireworks
                | Self::Cerebras
                | Self::Perplexity
                | Self::Cohere
                | Self::Sambanova
                | Self::Nvidia
                | Self::Github
                | Self::Moonshot
                | Self::Qwen
                | Self::Zhipu
                | Self::Azure
        )
    }

    pub fn is_local(&self) -> bool {
        matches!(self, Self::Ollama | Self::LmStudio)
    }
}

/// Provider configuration used when switching providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider_type: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub auth_type: Option<String>,
    pub auth_value: Option<String>,
    pub auth_header: Option<String>,
}

/// Provider info returned to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    #[serde(rename = "type")]
    pub provider_type: String,
    pub name: String,
    pub base_url: String,
    pub requires_api_key: bool,
    pub is_local: bool,
}

/// Routes to active LLM provider, supports hot-switching and cancellation.
pub struct LLMRouter {
    active_provider: Option<Arc<TokioMutex<Box<dyn LLMProvider>>>>,
    active_model: String,
    active_provider_type: Option<ProviderType>,
    cancel_token: Arc<TokioMutex<bool>>,
    credentials: Option<Arc<std::sync::Mutex<crate::credentials::CredentialManager>>>,
}

impl LLMRouter {
    pub fn new() -> Self {
        Self {
            active_provider: None,
            active_model: String::new(),
            active_provider_type: None,
            cancel_token: Arc::new(TokioMutex::new(false)),
            credentials: None,
        }
    }

    pub fn set_credentials(
        &mut self,
        credentials: Arc<std::sync::Mutex<crate::credentials::CredentialManager>>,
    ) {
        self.credentials = Some(credentials);
    }

    /// Set the active provider, creating the appropriate client.
    pub fn set_provider(
        &mut self,
        config: ProviderConfig,
    ) -> Result<(), LLMError> {
        let provider_type = ProviderType::from_str(&config.provider_type)?;

        let provider: Box<dyn LLMProvider> = match &provider_type {
            ProviderType::Ollama => {
                Box::new(ollama::OllamaClient::new(config.base_url.as_deref()))
            }
            ProviderType::LmStudio => {
                Box::new(openai_compat::create_lm_studio_client(
                    config.base_url.as_deref(),
                ))
            }
            ProviderType::Openai => {
                let api_key = config.api_key.as_deref().ok_or_else(|| {
                    LLMError::NotConfigured("OpenAI API key required".to_string())
                })?;
                Box::new(openai_compat::create_openai_client(api_key))
            }
            ProviderType::Anthropic => {
                let api_key = config.api_key.as_deref().ok_or_else(|| {
                    LLMError::NotConfigured("Anthropic API key required".to_string())
                })?;
                Box::new(anthropic::AnthropicClient::new(
                    api_key,
                    config.base_url.as_deref(),
                ))
            }
            ProviderType::Groq => {
                let api_key = config.api_key.as_deref().ok_or_else(|| {
                    LLMError::NotConfigured("Groq API key required".to_string())
                })?;
                Box::new(openai_compat::create_groq_client(api_key))
            }
            ProviderType::Gemini => {
                let api_key = config.api_key.as_deref().ok_or_else(|| {
                    LLMError::NotConfigured("Gemini API key required".to_string())
                })?;
                Box::new(gemini::GeminiClient::new(
                    api_key,
                    config.base_url.as_deref(),
                ))
            }
            ProviderType::Openrouter => {
                let api_key = config.api_key.as_deref().ok_or_else(|| {
                    LLMError::NotConfigured("OpenRouter API key required".to_string())
                })?;
                Box::new(openai_compat::create_openrouter_client(api_key))
            }
            ProviderType::Xai
            | ProviderType::Codex
            | ProviderType::Mistral
            | ProviderType::Deepseek
            | ProviderType::Together
            | ProviderType::Fireworks
            | ProviderType::Cerebras
            | ProviderType::Perplexity
            | ProviderType::Cohere
            | ProviderType::Sambanova
            | ProviderType::Nvidia
            | ProviderType::Github
            | ProviderType::Moonshot
            | ProviderType::Qwen
            | ProviderType::Zhipu => {
                let api_key = config.api_key.as_deref().ok_or_else(|| {
                    LLMError::NotConfigured(format!(
                        "{} API key required",
                        provider_type.display_name()
                    ))
                })?;
                Box::new(openai_compat::create_bearer_client(
                    provider_type.as_str(),
                    provider_type.default_base_url(),
                    api_key,
                ))
            }
            ProviderType::Azure => {
                let api_key = config.api_key.as_deref().ok_or_else(|| {
                    LLMError::NotConfigured("Azure OpenAI API key required".to_string())
                })?;
                let base = config.base_url.as_deref().ok_or_else(|| {
                    LLMError::NotConfigured("Azure OpenAI endpoint required".to_string())
                })?;
                Box::new(openai_compat::create_api_key_client(
                    "azure",
                    &openai_compat::normalize_azure_base_url(base),
                    api_key,
                    "api-key",
                ))
            }
            ProviderType::Chatgpt => {
                let access = config.api_key.ok_or_else(|| {
                    LLMError::NotConfigured(
                        "Sign in with ChatGPT before using the subscription".to_string(),
                    )
                })?;
                let session = chatgpt_auth::ChatGptSession {
                    access_token: access,
                    refresh_token: config.auth_value.unwrap_or_default(),
                    account_id: config.auth_header.unwrap_or_default(),
                };
                Box::new(chatgpt::ChatGptClient::new(session, self.credentials.clone()))
            }
            ProviderType::Custom => {
                let base_url = config.base_url.ok_or_else(|| {
                    LLMError::NotConfigured("Custom endpoint base URL required".to_string())
                })?;
                let auth_type = match config.auth_type.as_deref() {
                    Some("bearer") => custom::CustomAuthType::Bearer,
                    Some("api_key") | Some("api-key") => custom::CustomAuthType::ApiKey,
                    _ => custom::CustomAuthType::None,
                };
                Box::new(custom::CustomClient::new(custom::CustomConfig {
                    base_url,
                    auth_type,
                    auth_value: config.auth_value,
                    auth_header: config.auth_header,
                }))
            }
        };

        self.active_provider = Some(Arc::new(TokioMutex::new(provider)));
        self.active_provider_type = Some(provider_type);
        Ok(())
    }

    /// Get a reference to the active provider (behind a tokio mutex for async access).
    pub fn get_provider(&self) -> Result<Arc<TokioMutex<Box<dyn LLMProvider>>>, LLMError> {
        self.active_provider
            .clone()
            .ok_or_else(|| LLMError::NotConfigured("No LLM provider configured".to_string()))
    }

    /// Get the active model name.
    pub fn active_model(&self) -> &str {
        &self.active_model
    }

    /// Set the active model.
    pub fn set_active_model(&mut self, model: String) {
        self.active_model = model;
    }

    /// Get the active provider type.
    pub fn active_provider_type(&self) -> Option<&ProviderType> {
        self.active_provider_type.as_ref()
    }

    /// Request cancellation of the current stream.
    pub async fn cancel(&self) {
        let mut cancel = self.cancel_token.lock().await;
        *cancel = true;
    }

    /// Check if cancellation was requested and reset the flag.
    pub async fn check_cancelled(&self) -> bool {
        let mut cancel = self.cancel_token.lock().await;
        let was_cancelled = *cancel;
        if was_cancelled {
            *cancel = false;
        }
        was_cancelled
    }

    /// Get information about all available providers.
    pub fn get_all_providers() -> Vec<ProviderInfo> {
        let all_types = [
            ProviderType::Ollama,
            ProviderType::LmStudio,
            ProviderType::Openai,
            ProviderType::Anthropic,
            ProviderType::Groq,
            ProviderType::Gemini,
            ProviderType::Openrouter,
            ProviderType::Xai,
            ProviderType::Codex,
            ProviderType::Mistral,
            ProviderType::Deepseek,
            ProviderType::Together,
            ProviderType::Fireworks,
            ProviderType::Cerebras,
            ProviderType::Perplexity,
            ProviderType::Cohere,
            ProviderType::Sambanova,
            ProviderType::Nvidia,
            ProviderType::Github,
            ProviderType::Moonshot,
            ProviderType::Qwen,
            ProviderType::Zhipu,
            ProviderType::Chatgpt,
            ProviderType::Azure,
            ProviderType::Custom,
        ];

        all_types
            .iter()
            .map(|pt| ProviderInfo {
                provider_type: pt.as_str().to_string(),
                name: pt.display_name().to_string(),
                base_url: pt.default_base_url().to_string(),
                requires_api_key: pt.requires_api_key(),
                is_local: pt.is_local(),
            })
            .collect()
    }
}
