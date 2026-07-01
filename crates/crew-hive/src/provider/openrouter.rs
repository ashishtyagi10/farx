//! An [`Provider`] backed by OpenRouter's OpenAI-compatible chat-completions
//! API, so the inbuilt agents can run on any OpenRouter-hosted model via
//! `OPENROUTER_API_KEY`. Mirrors [`super::AnthropicProvider`] but speaks the
//! OpenAI request/response shape (a `messages` array, `choices[].message`).
use std::future::Future;
use std::pin::Pin;

use serde::Deserialize;

use super::{Completion, CompletionRequest, Provider, ProviderError};

const ENDPOINT: &str = "https://openrouter.ai/api/v1/chat/completions";
/// How many times to retry one model on a transient error before the chain
/// advances to the next model (kept low because the fallback chain adds breadth).
const MAX_RETRIES: u32 = 2;

/// Seconds to wait before retrying, or `None` to not retry. A call is treated as
/// transiently retryable when the HTTP status is 429/5xx *or* the body carries an
/// OpenRouter-wrapped upstream rate-limit error (it returns those as a 200 with
/// an `error` object of `"code":429`). Honors an explicit `Retry-After` header or
/// the body's `retry_after_seconds`, else backs off exponentially; clamped so a
/// hung retry loop can't outlast the agent call's own timeout.
fn retry_delay(status: u16, retry_after_hdr: Option<u64>, body: &str, attempt: u32) -> Option<u64> {
    let transient = status == 429
        || (500..600).contains(&status)
        || body.contains("\"code\":429")
        || body.contains("rate-limit")
        || body.contains("rate limit");
    if !transient {
        return None;
    }
    let body_hint = body
        .split("retry_after_seconds\":")
        .nth(1)
        .and_then(|s| s.split([',', '}']).next())
        .and_then(|s| s.trim().parse::<f64>().ok())
        .map(|f| f.ceil() as u64);
    Some(
        retry_after_hdr
            .or(body_hint)
            .unwrap_or(1u64 << attempt)
            .clamp(1, 8),
    )
}

/// Cheap to clone (the `reqwest::Client` is an `Arc` internally; the key is a
/// short `String`).
#[derive(Clone)]
pub struct OpenRouterProvider {
    client: reqwest::Client,
    api_key: String,
    /// Fallback model chain: when the requested model is rate-limited or
    /// unavailable, `complete` advances to the next slug here. Empty = no
    /// fallback (just the request's own model, with transient retry).
    fallbacks: Vec<String>,
}

/// The ordered, de-duplicated models to try for one request: the requested model
/// first, then each configured fallback not already present. Because free
/// OpenRouter models route to *different* upstream providers, a different slug
/// often dodges a provider-specific throttle even when the account-wide daily
/// cap is shared.
fn attempt_chain(primary: &str, fallbacks: &[String]) -> Vec<String> {
    let mut chain = vec![primary.to_string()];
    for m in fallbacks {
        if !m.is_empty() && !chain.contains(m) {
            chain.push(m.clone());
        }
    }
    chain
}

/// One model's request with transient-error retry (see [`retry_delay`]).
async fn request_with_retry(
    client: &reqwest::Client,
    key: &str,
    body: &serde_json::Value,
) -> Result<Completion, ProviderError> {
    let mut attempt = 0u32;
    loop {
        let resp = client
            .post(ENDPOINT)
            .header("authorization", format!("Bearer {key}"))
            .header("content-type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| ProviderError::Http(e.to_string()))?;
        let status = resp.status().as_u16();
        let retry_after_hdr = resp
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.trim().parse::<u64>().ok());
        let text = resp
            .text()
            .await
            .map_err(|e| ProviderError::Http(e.to_string()))?;
        if attempt < MAX_RETRIES {
            if let Some(wait) = retry_delay(status, retry_after_hdr, &text, attempt) {
                attempt += 1;
                tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
                continue;
            }
        }
        return OpenRouterProvider::parse_response(&text);
    }
}

#[derive(Deserialize, Default)]
struct Msg {
    #[serde(default)]
    content: String,
}

#[derive(Deserialize)]
struct Choice {
    #[serde(default)]
    message: Msg,
}

#[derive(Deserialize)]
struct Usage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
}

#[derive(Deserialize)]
struct ApiResp {
    #[serde(default)]
    choices: Vec<Choice>,
    usage: Option<Usage>,
    #[serde(default)]
    error: Option<serde_json::Value>,
}

impl OpenRouterProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            fallbacks: Vec::new(),
        }
    }

    /// Set the fallback model chain (see [`OpenRouterProvider::fallbacks`]). The
    /// request's own model is always tried first; these are the alternates.
    pub fn with_fallbacks(mut self, models: Vec<String>) -> Self {
        self.fallbacks = models;
        self
    }

    pub fn from_env() -> Result<Self, ProviderError> {
        match std::env::var("OPENROUTER_API_KEY") {
            Ok(k) if !k.is_empty() => Ok(Self::new(k)),
            _ => Err(ProviderError::MissingKey),
        }
    }

    pub(crate) fn parse_response(body: &str) -> Result<Completion, ProviderError> {
        let r: ApiResp =
            serde_json::from_str(body).map_err(|e| ProviderError::Decode(e.to_string()))?;
        if r.error.is_some() {
            return Err(ProviderError::Api(body.to_string()));
        }
        let text = r
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();
        let usage = r
            .usage
            .ok_or_else(|| ProviderError::Decode("missing usage".into()))?;
        Ok(Completion {
            text,
            input_tokens: usage.prompt_tokens,
            output_tokens: usage.completion_tokens,
        })
    }
}

impl Provider for OpenRouterProvider {
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        let client = self.client.clone();
        let key = self.api_key.clone();
        let chain = attempt_chain(&req.model, &self.fallbacks);
        Box::pin(async move {
            let mut messages = Vec::new();
            if let Some(sys) = &req.system {
                messages.push(serde_json::json!({"role": "system", "content": sys}));
            }
            messages.push(serde_json::json!({"role": "user", "content": req.prompt}));
            // Try each model in turn; a model that stays rate-limited or is
            // unavailable (retired free slug, upstream error) hands off to the
            // next. Only a missing key short-circuits — no model can fix that.
            let mut last_err = ProviderError::Api("no model attempted".into());
            for model in &chain {
                let body = serde_json::json!({
                    "model": model,
                    "max_tokens": req.max_tokens,
                    "messages": messages,
                });
                match request_with_retry(&client, &key, &body).await {
                    Ok(c) => return Ok(c),
                    Err(ProviderError::MissingKey) => return Err(ProviderError::MissingKey),
                    Err(e) => last_err = e,
                }
            }
            Err(last_err)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{attempt_chain, retry_delay};

    #[test]
    fn chain_puts_requested_model_first_then_fallbacks() {
        let fb = vec!["b:free".to_string(), "c:free".to_string()];
        assert_eq!(
            attempt_chain("a:free", &fb),
            vec!["a:free", "b:free", "c:free"]
        );
    }

    #[test]
    fn chain_dedups_and_skips_empty() {
        // The requested model also appearing in the fallbacks isn't tried twice,
        // and empty entries are dropped.
        let fb = vec!["a:free".to_string(), String::new(), "b:free".to_string()];
        assert_eq!(attempt_chain("a:free", &fb), vec!["a:free", "b:free"]);
    }

    #[test]
    fn chain_of_one_when_no_fallbacks() {
        assert_eq!(attempt_chain("only:free", &[]), vec!["only:free"]);
    }

    // The exact OpenRouter free-tier body the user hit: a 200 wrapping an
    // upstream 429 with a Retry-After hint in metadata.
    const RL_BODY: &str = r#"{"error":{"message":"Provider returned error","code":429,"metadata":{"raw":"... temporarily rate-limited upstream ...","retry_after_seconds":3.719}}}"#;

    #[test]
    fn retries_wrapped_429_using_body_hint() {
        // 200 status, but the body carries code 429 → retry, ceil(3.719)=4s.
        assert_eq!(retry_delay(200, None, RL_BODY, 0), Some(4));
    }

    #[test]
    fn retry_after_header_wins_and_clamps() {
        // Header present → used, then clamped into [1, 8].
        assert_eq!(retry_delay(429, Some(2), "{}", 0), Some(2));
        assert_eq!(retry_delay(429, Some(999), "{}", 0), Some(8));
    }

    #[test]
    fn exponential_backoff_when_no_hint() {
        assert_eq!(retry_delay(503, None, "", 0), Some(1));
        assert_eq!(retry_delay(503, None, "", 2), Some(4));
    }

    #[test]
    fn does_not_retry_hard_errors() {
        assert_eq!(
            retry_delay(400, None, r#"{"error":"bad request"}"#, 0),
            None
        );
        assert_eq!(retry_delay(401, None, "unauthorized", 0), None);
        assert_eq!(retry_delay(200, None, r#"{"choices":[]}"#, 0), None);
    }
}
