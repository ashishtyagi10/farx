use super::prompt::{build_system_prompt, not_configured_message};
use super::state::{AiAgent, ApiProvider};
use super::types::{
    AnthropicRequest, AnthropicResponse, ChatMessage, OpenAiRequest, OpenAiResponse,
};
use anyhow::Result;
use std::path::Path;

impl AiAgent {
    /// Process a natural language query about files.
    pub async fn query(
        &self,
        user_query: &str,
        current_dir: &Path,
        files_context: &str,
    ) -> Result<String> {
        let api_key = match &self.api_key {
            Some(key) => key.clone(),
            None => return Ok(not_configured_message(self.api_key_env_name())),
        };
        let system_prompt = build_system_prompt(current_dir, files_context);

        match self.provider {
            ApiProvider::Anthropic => {
                self.query_anthropic(&api_key, &system_prompt, user_query)
                    .await
            }
            ApiProvider::OpenAiCompatible => {
                self.query_openai_with_fallbacks(&api_key, &system_prompt, user_query)
                    .await
            }
        }
    }

    pub(super) async fn query_anthropic(
        &self,
        api_key: &str,
        system_prompt: &str,
        user_query: &str,
    ) -> Result<String> {
        let request = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            system: system_prompt.to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: user_query.to_string(),
            }],
        };
        let url = format!("{}/messages", self.base_url.trim_end_matches('/'));
        let response = reqwest::Client::new()
            .post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Ok(format!("API error ({}): {}", status, body));
        }
        let msg: AnthropicResponse = response.json().await?;
        let text = msg
            .content
            .iter()
            .filter_map(|b| b.text.as_ref())
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        Ok(if text.is_empty() {
            "No response from AI.".into()
        } else {
            text
        })
    }

    /// Try the primary model first, then each fallback on 429/5xx. Returns
    /// the first successful response, or the last error.
    pub(super) async fn query_openai_with_fallbacks(
        &self,
        api_key: &str,
        system_prompt: &str,
        user_query: &str,
    ) -> Result<String> {
        let mut models: Vec<&str> = vec![self.model.as_str()];
        models.extend(self.fallback_models.iter().map(|s| s.as_str()));

        let mut last_error: Option<String> = None;
        for model in models {
            match self
                .try_openai_model(api_key, system_prompt, user_query, model)
                .await
            {
                Ok(text) => return Ok(text),
                Err(e) => last_error = Some(format!("{}: {}", model, e)),
            }
        }
        Ok(format!(
            "All AI models failed. Last error — {}",
            last_error.unwrap_or_else(|| "no fallbacks configured".into())
        ))
    }

    /// One attempt at a single OpenAI-compatible model. Falls back internally
    /// from system+user messages to a merged user message if the server
    /// rejects the first shape. Errors with the body text on non-2xx so the
    /// caller can decide whether to try a different model.
    async fn try_openai_model(
        &self,
        api_key: &str,
        system_prompt: &str,
        user_query: &str,
        model: &str,
    ) -> Result<String> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let client = reqwest::Client::new();

        let with_system = vec![
            ChatMessage {
                role: "system".into(),
                content: system_prompt.into(),
            },
            ChatMessage {
                role: "user".into(),
                content: user_query.into(),
            },
        ];
        let response =
            post_openai(&client, &url, api_key, model, self.max_tokens, with_system).await?;

        if response.status().is_success() {
            return extract_openai_text(response).await;
        }
        let status = response.status();
        // Surface 429 / 5xx to caller so they can try the next fallback.
        if status.as_u16() == 429 || status.is_server_error() {
            anyhow::bail!("HTTP {}", status);
        }
        // Other non-success: retry once with merged messages (some endpoints
        // reject "system" role).
        let merged = vec![ChatMessage {
            role: "user".into(),
            content: format!("{}\n\nUser request: {}", system_prompt, user_query),
        }];
        let retry = post_openai(&client, &url, api_key, model, self.max_tokens, merged).await?;
        if !retry.status().is_success() {
            let s = retry.status();
            anyhow::bail!("HTTP {}", s);
        }
        extract_openai_text(retry).await
    }
}

async fn post_openai(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    model: &str,
    max_tokens: u32,
    messages: Vec<ChatMessage>,
) -> Result<reqwest::Response> {
    let request = OpenAiRequest {
        model: model.into(),
        max_tokens,
        messages,
    };
    let resp = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("content-type", "application/json")
        .header("HTTP-Referer", "https://github.com/farx-fm/farx")
        .header("X-Title", "Farx File Manager")
        .json(&request)
        .send()
        .await?;
    Ok(resp)
}

async fn extract_openai_text(response: reqwest::Response) -> Result<String> {
    let msg: OpenAiResponse = response.json().await?;
    let text = msg
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_default();
    Ok(if text.is_empty() {
        "No response from AI.".into()
    } else {
        text
    })
}
