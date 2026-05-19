use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiConfig {
    pub enabled: bool,
    /// "anthropic", "openrouter", or "openai-compatible"
    pub provider: String,
    /// Base URL for the API (e.g. "https://openrouter.ai/api/v1")
    pub base_url: String,
    pub model: String,
    /// Models to try in order when the primary returns 429 or 5xx. Useful
    /// for OpenRouter `:free` models which share a global rate-limit pool.
    pub fallback_models: Vec<String>,
    pub max_tokens: u32,
    /// Environment variable name to read the API key from
    pub api_key_env: String,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "openrouter".to_string(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
            model: "openai/gpt-oss-20b:free".to_string(),
            fallback_models: default_free_fallbacks(),
            max_tokens: 4096,
            api_key_env: "OPENROUTER_API_KEY".to_string(),
        }
    }
}

/// Sensible OpenRouter free-tier fallback chain. Order picks lighter / more
/// widely-available models first, then heavier alternates.
fn default_free_fallbacks() -> Vec<String> {
    vec![
        "openai/gpt-oss-120b:free".to_string(),
        "google/gemma-4-31b-it:free".to_string(),
        "deepseek/deepseek-v4-flash:free".to_string(),
        "qwen/qwen3-coder:free".to_string(),
        "meta-llama/llama-3.3-70b-instruct:free".to_string(),
    ]
}
