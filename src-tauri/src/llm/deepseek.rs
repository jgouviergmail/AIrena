//! DeepSeek provider — OpenAI-compatible chat completions over SSE.
//!
//! Verified against the official API reference (2026-09-10):
//! - `thinking: {type}` + `reasoning_effort` control reasoning; while thinking is
//!   enabled `temperature` is unsupported and `top_p` is floored at 0.95.
//! - `reasoning_content` streams in `delta` before `content`.
//! - `stream_options.include_usage` puts a `usage` object in the last chunk.
//! - Under load the server sends `: keep-alive` SSE comments; the idle timeout
//!   is reset by any traffic.
//! - `max_tokens` is always sent explicitly (API defaults reach 64K/128K when
//!   thinking, which would be a cost hazard).

use std::fmt;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use super::{LlmCapabilities, LlmError, LlmProvider, LlmRequest, LlmResponse, TokenCallback};
use crate::constants;
use crate::models::llm::{LlmUsage, ProviderKind, ReasoningLevel};

// ── Wire types ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct WireMessage {
    pub role: &'static str,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct StreamOptions {
    pub include_usage: bool,
}

#[derive(Debug, Serialize)]
pub struct ResponseFormat {
    #[serde(rename = "type")]
    pub kind: &'static str,
}

#[derive(Debug, Serialize)]
pub struct Thinking {
    #[serde(rename = "type")]
    pub kind: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<WireMessage>,
    pub stream: bool,
    pub stream_options: StreamOptions,
    pub max_tokens: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    pub thinking: Thinking,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<&'static str>,
}

#[derive(Debug, Default, Deserialize)]
struct Delta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    reasoning_content: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct StreamChoice {
    #[serde(default)]
    delta: Delta,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct PromptTokensDetails {
    #[serde(default)]
    cached_tokens: Option<u32>,
    #[serde(default)]
    prompt_cache_hit_tokens: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
struct CompletionTokensDetails {
    #[serde(default)]
    reasoning_tokens: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
struct WireUsage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
    #[serde(default)]
    prompt_tokens_details: Option<PromptTokensDetails>,
    #[serde(default)]
    completion_tokens_details: Option<CompletionTokensDetails>,
    /// Legacy top-level field kept by the API for compatibility.
    #[serde(default)]
    prompt_cache_hit_tokens: Option<u32>,
}

impl From<WireUsage> for LlmUsage {
    fn from(u: WireUsage) -> Self {
        let cached = u
            .prompt_tokens_details
            .as_ref()
            .and_then(|d| d.cached_tokens.or(d.prompt_cache_hit_tokens))
            .or(u.prompt_cache_hit_tokens)
            .unwrap_or(0);
        Self {
            prompt_tokens: u.prompt_tokens,
            cached_tokens: cached.min(u.prompt_tokens),
            completion_tokens: u.completion_tokens,
            reasoning_tokens: u
                .completion_tokens_details
                .and_then(|d| d.reasoning_tokens)
                .unwrap_or(0)
                .min(u.completion_tokens),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct StreamChunk {
    #[serde(default)]
    choices: Vec<StreamChoice>,
    #[serde(default)]
    usage: Option<WireUsage>,
}

#[derive(Debug, Deserialize)]
struct ApiErrorDetail {
    #[serde(default)]
    message: String,
}

#[derive(Debug, Deserialize)]
struct ApiErrorBody {
    error: ApiErrorDetail,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    id: String,
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    #[serde(default)]
    data: Vec<ModelEntry>,
}

/// Balance information from `GET /user/balance` (sent to the frontend).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeepSeekBalance {
    pub is_available: bool,
    #[serde(default)]
    pub balances: Vec<BalanceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceInfo {
    pub currency: String,
    pub total_balance: String,
}

#[derive(Debug, Deserialize)]
struct WireBalanceInfo {
    #[serde(default)]
    currency: String,
    #[serde(default)]
    total_balance: String,
}

#[derive(Debug, Deserialize)]
struct WireBalance {
    #[serde(default)]
    is_available: bool,
    #[serde(default)]
    balance_infos: Vec<WireBalanceInfo>,
}

// ── Provider ────────────────────────────────────────────────────────────

pub struct DeepSeekProvider {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
    caps: LlmCapabilities,
}

impl fmt::Debug for DeepSeekProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeepSeekProvider")
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("api_key", &"***")
            .finish()
    }
}

/// Outcome of one streaming attempt: the error plus whether tokens were already
/// forwarded (a retry would duplicate them).
type AttemptError = (LlmError, bool);

impl DeepSeekProvider {
    pub fn new(api_key: &str, model: &str, context_budget: u32) -> Result<Self, LlmError> {
        Self::with_base_url(constants::DEEPSEEK_BASE_URL, api_key, model, context_budget)
    }

    /// Constructor with a custom base URL (tests against a local mock server).
    pub fn with_base_url(base_url: &str, api_key: &str, model: &str, context_budget: u32) -> Result<Self, LlmError> {
        let api_key = api_key.trim();
        if api_key.is_empty() {
            return Err(LlmError::Auth);
        }
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(constants::DEEPSEEK_CONNECT_TIMEOUT_SECS))
            .build()
            .map_err(|e| LlmError::Connection(format!("HTTP client build failed: {e}")))?;
        let model = if model.trim().is_empty() {
            constants::DEEPSEEK_DEFAULT_MODEL.to_string()
        } else {
            model.trim().to_string()
        };
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            model,
            caps: LlmCapabilities {
                supports_reasoning: true,
                reasoning_levels: true,
                reasoning_displayable: true,
                supports_json_mode: true,
                context_tokens: context_budget,
                chars_per_token_latin: constants::DEEPSEEK_CHARS_PER_TOKEN_LATIN,
                chars_per_token_cjk: constants::DEEPSEEK_CHARS_PER_TOKEN_CJK,
                reports_usage: true,
                billable: true,
            },
        })
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn effort_name(level: ReasoningLevel) -> Option<&'static str> {
        match level {
            ReasoningLevel::Low => Some("low"),
            ReasoningLevel::High => Some("high"),
            ReasoningLevel::Max => Some("max"),
            ReasoningLevel::Off | ReasoningLevel::Auto => None,
        }
    }

    fn reasoning_allowance(level: ReasoningLevel) -> i32 {
        match level {
            ReasoningLevel::Low => constants::DEEPSEEK_REASONING_ALLOWANCE_LOW,
            ReasoningLevel::High => constants::DEEPSEEK_REASONING_ALLOWANCE_HIGH,
            ReasoningLevel::Max => constants::DEEPSEEK_REASONING_ALLOWANCE_MAX,
            ReasoningLevel::Off | ReasoningLevel::Auto => 0,
        }
    }

    /// Translate a generic request into the chat-completions body.
    pub fn to_wire(&self, request: &LlmRequest) -> ChatCompletionRequest {
        let thinking = request.reasoning.is_active();
        let params = &request.params;

        let mut messages = Vec::with_capacity(2);
        if !request.system.is_empty() {
            messages.push(WireMessage { role: "system", content: request.system.clone() });
        }
        messages.push(WireMessage { role: "user", content: request.user.clone() });

        let max_tokens = params
            .num_predict
            .saturating_add(Self::reasoning_allowance(request.reasoning))
            .clamp(1, constants::DEEPSEEK_MAX_OUTPUT_TOKENS);

        ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            stream: true,
            stream_options: StreamOptions { include_usage: true },
            max_tokens,
            // Unsupported while thinking — omitted rather than rejected by the API.
            temperature: (!thinking).then_some(params.temperature),
            top_p: Some(if thinking {
                params.top_p.max(constants::DEEPSEEK_TOP_P_MIN_THINKING)
            } else {
                params.top_p
            }),
            response_format: request.json_mode.then_some(ResponseFormat { kind: "json_object" }),
            thinking: Thinking { kind: if thinking { "enabled" } else { "disabled" } },
            reasoning_effort: if thinking { Self::effort_name(request.reasoning) } else { None },
        }
    }

    /// Map an HTTP error response to `LlmError` (body is the API's JSON error).
    fn classify_http_error(status: u16, body: &str) -> LlmError {
        let message = serde_json::from_str::<ApiErrorBody>(body)
            .map(|b| b.error.message)
            .unwrap_or_else(|_| body.chars().take(200).collect());
        let lower = message.to_lowercase();
        match status {
            401 => LlmError::Auth,
            402 => LlmError::InsufficientBalance,
            429 => LlmError::RateLimited,
            503 => LlmError::Overloaded,
            400 | 404 if lower.contains("model") && (lower.contains("not exist") || lower.contains("not found")) => {
                LlmError::ModelNotFound(message)
            }
            400 | 404 | 422 => LlmError::Client(format!("HTTP {status}: {message}")),
            500..=599 => LlmError::Connection(format!("HTTP {status}: {message}")),
            _ => LlmError::Client(format!("HTTP {status}: {message}")),
        }
    }

    async fn stream_once(
        &self,
        wire: &ChatCompletionRequest,
        on_content: TokenCallback<'_>,
        on_reasoning: TokenCallback<'_>,
        cancel: &CancellationToken,
    ) -> Result<LlmResponse, AttemptError> {
        let send = self
            .http
            .post(self.url(constants::DEEPSEEK_CHAT_PATH))
            .bearer_auth(&self.api_key)
            .json(wire)
            .send();

        let response = tokio::select! {
            r = send => r.map_err(|e| (LlmError::Connection(e.to_string()), false))?,
            _ = cancel.cancelled() => return Err((LlmError::Cancelled, false)),
        };

        let status = response.status().as_u16();
        if status != 200 {
            let body = response.text().await.unwrap_or_default();
            return Err((Self::classify_http_error(status, &body), false));
        }

        let mut stream = response.bytes_stream();
        let mut buf: Vec<u8> = Vec::new();
        let mut content = String::new();
        let mut reasoning = String::new();
        let mut usage: Option<LlmUsage> = None;
        let mut truncated = false;
        let mut emitted = false;
        let idle = Duration::from_secs(constants::DEEPSEEK_IDLE_TIMEOUT_SECS);

        loop {
            let next = tokio::select! {
                n = tokio::time::timeout(idle, stream.next()) => n,
                _ = cancel.cancelled() => return Err((LlmError::Cancelled, emitted)),
            };
            let chunk = match next {
                Err(_) => {
                    return Err((
                        LlmError::Connection(format!("No data for {}s (idle timeout)", constants::DEEPSEEK_IDLE_TIMEOUT_SECS)),
                        emitted,
                    ))
                }
                Ok(None) => break, // EOF without [DONE]: accept what we have
                Ok(Some(Err(e))) => return Err((LlmError::Connection(e.to_string()), emitted)),
                Ok(Some(Ok(bytes))) => bytes,
            };
            buf.extend_from_slice(&chunk);

            let mut done = false;
            while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                let line: Vec<u8> = buf.drain(..=pos).collect();
                let line = String::from_utf8_lossy(&line);
                match parse_sse_line(line.trim()) {
                    SseLine::Skip => {}
                    SseLine::Done => {
                        done = true;
                        break;
                    }
                    SseLine::Data(payload) => {
                        let parsed: StreamChunk = serde_json::from_str(payload)
                            .map_err(|e| (LlmError::Json(format!("bad SSE chunk: {e}")), emitted))?;
                        if let Some(u) = parsed.usage {
                            usage = Some(u.into());
                        }
                        for choice in parsed.choices {
                            if let Some(r) = choice.delta.reasoning_content.filter(|s| !s.is_empty()) {
                                on_reasoning(&r);
                                reasoning.push_str(&r);
                                emitted = true;
                            }
                            if let Some(c) = choice.delta.content.filter(|s| !s.is_empty()) {
                                on_content(&c);
                                content.push_str(&c);
                                emitted = true;
                            }
                            match choice.finish_reason.as_deref() {
                                Some("length") => truncated = true,
                                Some("insufficient_system_resource") | Some("aborted") => {
                                    return Err((LlmError::Overloaded, emitted))
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            if done {
                break;
            }
        }

        if truncated {
            tracing::warn!(chars = content.len(), model = %self.model, "DeepSeek response truncated (max_tokens reached)");
        }
        Ok(LlmResponse {
            content: content.trim().to_string(),
            reasoning: (!reasoning.is_empty()).then_some(reasoning),
            usage,
            truncated,
        })
    }

    /// `GET /models` → model ids. Falls back to the documented list on failure.
    pub async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        let resp = self
            .http
            .get(self.url(constants::DEEPSEEK_MODELS_PATH))
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|e| LlmError::Connection(e.to_string()))?;
        let status = resp.status().as_u16();
        if status != 200 {
            let body = resp.text().await.unwrap_or_default();
            return Err(Self::classify_http_error(status, &body));
        }
        let parsed: ModelsResponse = resp
            .json()
            .await
            .map_err(|e| LlmError::Json(format!("models response: {e}")))?;
        let mut ids: Vec<String> = parsed.data.into_iter().map(|m| m.id).collect();
        if ids.is_empty() {
            ids = constants::DEEPSEEK_KNOWN_MODELS.iter().map(|s| s.to_string()).collect();
        }
        Ok(ids)
    }

    /// `GET /user/balance` — also the cheapest way to validate an API key.
    pub async fn balance(&self) -> Result<DeepSeekBalance, LlmError> {
        let resp = self
            .http
            .get(self.url(constants::DEEPSEEK_BALANCE_PATH))
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|e| LlmError::Connection(e.to_string()))?;
        let status = resp.status().as_u16();
        if status != 200 {
            let body = resp.text().await.unwrap_or_default();
            return Err(Self::classify_http_error(status, &body));
        }
        let parsed: WireBalance = resp
            .json()
            .await
            .map_err(|e| LlmError::Json(format!("balance response: {e}")))?;
        Ok(DeepSeekBalance {
            is_available: parsed.is_available,
            balances: parsed
                .balance_infos
                .into_iter()
                .map(|b| BalanceInfo { currency: b.currency, total_balance: b.total_balance })
                .collect(),
        })
    }
}

enum SseLine<'a> {
    Skip,
    Done,
    Data(&'a str),
}

/// Parse one SSE line: comments (`: keep-alive`), empty lines and non-data
/// fields are skipped; `data: [DONE]` ends the stream.
fn parse_sse_line(line: &str) -> SseLine<'_> {
    if line.is_empty() || line.starts_with(':') {
        return SseLine::Skip;
    }
    let Some(payload) = line.strip_prefix("data:") else {
        return SseLine::Skip;
    };
    let payload = payload.trim();
    if payload == "[DONE]" {
        SseLine::Done
    } else if payload.is_empty() {
        SseLine::Skip
    } else {
        SseLine::Data(payload)
    }
}

fn backoff_delay(attempt: u32) -> Duration {
    use rand::Rng;
    let base = constants::DEEPSEEK_RETRY_BASE_MS.saturating_mul(1u64 << attempt.min(6));
    let jitter = rand::thread_rng().gen_range(0..=constants::DEEPSEEK_RETRY_JITTER_MS);
    Duration::from_millis(base + jitter)
}

#[async_trait]
impl LlmProvider for DeepSeekProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::DeepSeek
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn capabilities(&self) -> &LlmCapabilities {
        &self.caps
    }

    async fn chat_stream(
        &self,
        request: &LlmRequest,
        on_content: TokenCallback<'_>,
        on_reasoning: TokenCallback<'_>,
        cancel: CancellationToken,
    ) -> Result<LlmResponse, LlmError> {
        let wire = self.to_wire(request);
        let mut attempt = 0u32;
        loop {
            match self.stream_once(&wire, on_content, on_reasoning, &cancel).await {
                Ok(r) => return Ok(r),
                Err((e, emitted)) => {
                    let retry = e.is_retryable() && !emitted && attempt < constants::DEEPSEEK_MAX_RETRIES;
                    if !retry {
                        return Err(e);
                    }
                    let delay = backoff_delay(attempt);
                    tracing::warn!(error = %e, attempt = attempt + 1, delay_ms = delay.as_millis() as u64, "DeepSeek transient error — retrying");
                    tokio::select! {
                        _ = tokio::time::sleep(delay) => {}
                        _ = cancel.cancelled() => return Err(LlmError::Cancelled),
                    }
                    attempt += 1;
                }
            }
        }
    }

    async fn validate(&self) -> Result<(), LlmError> {
        let balance = self.balance().await?;
        if !balance.is_available {
            return Err(LlmError::InsufficientBalance);
        }
        let models = self.list_models().await?;
        if !models.iter().any(|m| m == &self.model) {
            return Err(LlmError::ModelNotFound(self.model.clone()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::llm::CallKind;
    use crate::models::settings::LlmParams;

    fn provider() -> DeepSeekProvider {
        DeepSeekProvider::new("sk-test", "deepseek-flash", 32_768).unwrap()
    }

    // ── Construction ───────────────────────────────────────────────────

    #[test]
    fn new_rejects_empty_key_and_defaults_model() {
        assert!(matches!(DeepSeekProvider::new("   ", "x", 1).unwrap_err(), LlmError::Auth));
        let p = DeepSeekProvider::new("sk-abc", "", 4096).unwrap();
        assert_eq!(p.model_name(), constants::DEEPSEEK_DEFAULT_MODEL);
        assert_eq!(p.kind(), ProviderKind::DeepSeek);
        assert!(p.capabilities().billable);
        assert!(p.capabilities().reasoning_levels);
        assert_eq!(p.capabilities().context_tokens, 4096);
    }

    #[test]
    fn debug_masks_api_key() {
        let dbg = format!("{:?}", DeepSeekProvider::new("sk-secret-123", "m", 1).unwrap());
        assert!(!dbg.contains("sk-secret-123"));
        assert!(dbg.contains("***"));
    }

    // ── Wire mapping ───────────────────────────────────────────────────

    #[test]
    fn wire_without_reasoning_sends_temperature_and_disables_thinking() {
        let params = LlmParams { temperature: 0.7, top_p: 0.5, num_predict: 1000, ..Default::default() };
        let req = LlmRequest::new("SYS", "USER", &params, CallKind::Reaction).json();
        let json = serde_json::to_value(provider().to_wire(&req)).unwrap();

        assert_eq!(json["model"], "deepseek-flash");
        assert_eq!(json["stream"], true);
        assert_eq!(json["stream_options"]["include_usage"], true);
        assert_eq!(json["max_tokens"], 1000);
        // JSON mode pins the structured-output temperature, not the persona's
        assert_eq!(json["temperature"], constants::TEMP_JSON_OUTPUT);
        assert_eq!(json["top_p"], 0.5f32);
        assert_eq!(json["thinking"]["type"], "disabled");
        assert!(json.get("reasoning_effort").is_none());
        assert_eq!(json["response_format"]["type"], "json_object");
        assert_eq!(json["messages"][0]["role"], "system");
        assert_eq!(json["messages"][1]["content"], "USER");
        // Ollama-only knobs never leak
        assert!(json.get("top_k").is_none());
        assert!(json.get("num_ctx").is_none());
        assert!(json.get("repeat_penalty").is_none());
    }

    #[test]
    fn wire_with_reasoning_omits_temperature_floors_top_p_and_adds_allowance() {
        let params = LlmParams { temperature: 0.9, top_p: 0.5, num_predict: 1000, ..Default::default() };
        for (level, effort, allowance) in [
            (ReasoningLevel::Low, "low", constants::DEEPSEEK_REASONING_ALLOWANCE_LOW),
            (ReasoningLevel::High, "high", constants::DEEPSEEK_REASONING_ALLOWANCE_HIGH),
            (ReasoningLevel::Max, "max", constants::DEEPSEEK_REASONING_ALLOWANCE_MAX),
        ] {
            let req = LlmRequest::new("s", "u", &params, CallKind::Intervention).reasoning(level);
            let json = serde_json::to_value(provider().to_wire(&req)).unwrap();
            assert_eq!(json["thinking"]["type"], "enabled", "{level:?}");
            assert_eq!(json["reasoning_effort"], effort, "{level:?}");
            assert!(json.get("temperature").is_none(), "{level:?}");
            assert_eq!(json["top_p"], constants::DEEPSEEK_TOP_P_MIN_THINKING, "{level:?}");
            assert_eq!(json["max_tokens"], 1000 + allowance, "{level:?}");
            assert!(json.get("response_format").is_none());
        }
        // top_p above the floor is kept
        let params = LlmParams { top_p: 0.99, ..Default::default() };
        let req = LlmRequest::new("s", "u", &params, CallKind::Intervention).reasoning(ReasoningLevel::Low);
        let json = serde_json::to_value(provider().to_wire(&req)).unwrap();
        assert_eq!(json["top_p"], 0.99f32);
    }

    #[test]
    fn wire_clamps_max_tokens_to_api_limit() {
        let params = LlmParams { num_predict: i32::MAX, ..Default::default() };
        let req = LlmRequest::new("s", "u", &params, CallKind::Intervention).reasoning(ReasoningLevel::Max);
        assert_eq!(provider().to_wire(&req).max_tokens, constants::DEEPSEEK_MAX_OUTPUT_TOKENS);
        let params = LlmParams { num_predict: 0, ..Default::default() };
        assert_eq!(provider().to_wire(&LlmRequest::new("s", "u", &params, CallKind::Vote)).max_tokens, 1);
    }

    // ── Error classification ───────────────────────────────────────────

    #[test]
    fn classify_http_errors() {
        let body = r#"{"error":{"message":"Model Not Exist","type":"invalid_request_error"}}"#;
        assert!(matches!(DeepSeekProvider::classify_http_error(400, body), LlmError::ModelNotFound(_)));
        assert!(matches!(DeepSeekProvider::classify_http_error(400, r#"{"error":{"message":"bad json"}}"#), LlmError::Client(_)));
        assert!(matches!(DeepSeekProvider::classify_http_error(401, ""), LlmError::Auth));
        assert!(matches!(DeepSeekProvider::classify_http_error(402, ""), LlmError::InsufficientBalance));
        assert!(matches!(DeepSeekProvider::classify_http_error(422, "{}"), LlmError::Client(_)));
        assert!(matches!(DeepSeekProvider::classify_http_error(429, ""), LlmError::RateLimited));
        assert!(matches!(DeepSeekProvider::classify_http_error(500, "oops"), LlmError::Connection(_)));
        assert!(matches!(DeepSeekProvider::classify_http_error(503, ""), LlmError::Overloaded));
        // Non-JSON body is truncated into the message, never panics
        let long = "x".repeat(5000);
        assert!(matches!(DeepSeekProvider::classify_http_error(418, &long), LlmError::Client(m) if m.len() < 300));
    }

    // ── SSE parsing ────────────────────────────────────────────────────

    #[test]
    fn sse_line_parsing() {
        assert!(matches!(parse_sse_line(""), SseLine::Skip));
        assert!(matches!(parse_sse_line(": keep-alive"), SseLine::Skip));
        assert!(matches!(parse_sse_line("event: ping"), SseLine::Skip));
        assert!(matches!(parse_sse_line("data:"), SseLine::Skip));
        assert!(matches!(parse_sse_line("data: [DONE]"), SseLine::Done));
        assert!(matches!(parse_sse_line("data:[DONE]"), SseLine::Done));
        assert!(matches!(parse_sse_line("data: {\"a\":1}"), SseLine::Data("{\"a\":1}")));
    }

    #[test]
    fn usage_conversion_prefers_details_and_clamps() {
        let wire: WireUsage = serde_json::from_str(
            r#"{"prompt_tokens":100,"completion_tokens":40,"prompt_tokens_details":{"cached_tokens":30},"completion_tokens_details":{"reasoning_tokens":12},"prompt_cache_hit_tokens":99}"#,
        )
        .unwrap();
        let u: LlmUsage = wire.into();
        assert_eq!(u, LlmUsage { prompt_tokens: 100, cached_tokens: 30, completion_tokens: 40, reasoning_tokens: 12 });

        // Legacy top-level cache field only
        let wire: WireUsage = serde_json::from_str(r#"{"prompt_tokens":10,"completion_tokens":5,"prompt_cache_hit_tokens":4}"#).unwrap();
        assert_eq!(LlmUsage::from(wire).cached_tokens, 4);

        // Clamped when inconsistent
        let wire: WireUsage = serde_json::from_str(r#"{"prompt_tokens":10,"completion_tokens":5,"prompt_tokens_details":{"cached_tokens":50},"completion_tokens_details":{"reasoning_tokens":50}}"#).unwrap();
        let u: LlmUsage = wire.into();
        assert_eq!(u.cached_tokens, 10);
        assert_eq!(u.reasoning_tokens, 5);
    }

    #[test]
    fn stream_chunk_deserialises_reasoning_then_content_and_final_usage() {
        let c1: StreamChunk = serde_json::from_str(r#"{"id":"1","choices":[{"index":0,"delta":{"role":"assistant","reasoning_content":"Let me"},"finish_reason":null}]}"#).unwrap();
        assert_eq!(c1.choices[0].delta.reasoning_content.as_deref(), Some("Let me"));
        let c2: StreamChunk = serde_json::from_str(r#"{"choices":[{"delta":{"content":"Bonjour"},"finish_reason":"length"}]}"#).unwrap();
        assert_eq!(c2.choices[0].delta.content.as_deref(), Some("Bonjour"));
        assert_eq!(c2.choices[0].finish_reason.as_deref(), Some("length"));
        let c3: StreamChunk = serde_json::from_str(r#"{"choices":[],"usage":{"prompt_tokens":7,"completion_tokens":3}}"#).unwrap();
        assert!(c3.choices.is_empty());
        assert_eq!(c3.usage.unwrap().prompt_tokens, 7);
    }

    #[test]
    fn backoff_grows_and_is_capped() {
        let d0 = backoff_delay(0).as_millis() as u64;
        let d2 = backoff_delay(2).as_millis() as u64;
        let d20 = backoff_delay(20).as_millis() as u64;
        assert!((constants::DEEPSEEK_RETRY_BASE_MS..=constants::DEEPSEEK_RETRY_BASE_MS + constants::DEEPSEEK_RETRY_JITTER_MS).contains(&d0));
        assert!(d2 >= constants::DEEPSEEK_RETRY_BASE_MS * 4);
        assert!(d20 <= constants::DEEPSEEK_RETRY_BASE_MS * 64 + constants::DEEPSEEK_RETRY_JITTER_MS);
    }

    // ── Lot 0 spike (needs DEEPSEEK_API_KEY) ───────────────────────────

    /// Validates H-DS-1 (`max_tokens` bounds reasoning + content) and JSON mode
    /// with/without thinking against the live API. Run manually:
    /// `DEEPSEEK_API_KEY=sk-... cargo test deepseek_live_spike -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn deepseek_live_spike() {
        let key = std::env::var("DEEPSEEK_API_KEY").expect("DEEPSEEK_API_KEY");
        let p = DeepSeekProvider::new(&key, constants::DEEPSEEK_DEFAULT_MODEL, 8192).unwrap();
        p.validate().await.expect("validate");

        // (a) tiny max_tokens with reasoning: does the API stop the reasoning too?
        let params = LlmParams { num_predict: 300, ..Default::default() };
        let mut req = LlmRequest::new("", "Explique en 3 phrases pourquoi le ciel est bleu.", &params, CallKind::Intervention)
            .reasoning(ReasoningLevel::Low);
        req.params.num_predict = 300;
        let r = p.chat(&req, CancellationToken::new()).await.expect("call a");
        println!("[a] truncated={} usage={:?} reasoning_len={} content_len={}", r.truncated, r.usage, r.reasoning.as_deref().map(str::len).unwrap_or(0), r.content.len());

        // (b) JSON mode without thinking
        let req = LlmRequest::new("Réponds en JSON.", "Donne {\"ok\": true} en json.", &params, CallKind::Reaction).json();
        let r = p.chat(&req, CancellationToken::new()).await.expect("call b");
        println!("[b] json/off content={}", r.content);

        // (c) JSON mode with thinking
        let req = LlmRequest::new("Réponds en JSON.", "Donne {\"ok\": true} en json.", &params, CallKind::Reaction).json().reasoning(ReasoningLevel::Low);
        let r = p.chat(&req, CancellationToken::new()).await.expect("call c");
        println!("[c] json/low content={} reasoning={:?}", r.content, r.reasoning.map(|s| s.len()));
    }
}

/// Transport-level tests against a throw-away local HTTP server (raw TCP, no
/// extra dependency): what the real API sends on the wire, including the
/// keep-alive comments and connection drops the unit tests cannot exercise.
#[cfg(test)]
mod transport_tests {
    use super::*;
    use crate::models::llm::CallKind;
    use crate::models::settings::LlmParams;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    /// Serve one HTTP response (status + body) for the next connection, then stop.
    /// `chunked_body` is written as separate TCP writes with a pause in between so
    /// the client really sees several SSE frames.
    async fn serve_once(status: &'static str, chunked_body: Vec<&'static str>, drop_before_end: bool) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            // Read the request head (and body) — enough to unblock the client
            let mut buf = vec![0u8; 16 * 1024];
            let _ = socket.read(&mut buf).await;
            let head = format!("HTTP/1.1 {status}\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n");
            socket.write_all(head.as_bytes()).await.unwrap();
            for part in chunked_body {
                socket.write_all(part.as_bytes()).await.unwrap();
                socket.flush().await.unwrap();
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            if !drop_before_end {
                socket.write_all(b"data: [DONE]\n\n").await.unwrap();
            }
            // Dropping the socket closes the connection (simulates a cut when no [DONE] was sent)
        });
        format!("http://{addr}")
    }

    fn request() -> LlmRequest {
        LlmRequest::new("SYS", "USER", &LlmParams::default(), CallKind::Intervention).reasoning(ReasoningLevel::Low)
    }

    #[tokio::test]
    async fn streams_keep_alive_reasoning_content_and_final_usage() {
        let base = serve_once(
            "200 OK",
            vec![
                ": keep-alive\n\n",
                "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"Je réfléchis. \"},\"finish_reason\":null}]}\n\n",
                ": keep-alive\n\n",
                "data: {\"choices\":[{\"delta\":{\"content\":\"Bonjour \"},\"finish_reason\":null}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"le monde\"},\"finish_reason\":\"stop\"}]}\n\n",
                "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":42,\"completion_tokens\":9,\"prompt_tokens_details\":{\"cached_tokens\":40},\"completion_tokens_details\":{\"reasoning_tokens\":4}}}\n\n",
            ],
            false,
        )
        .await;
        let provider = DeepSeekProvider::with_base_url(&base, "sk-test", "deepseek-flash", 8192).unwrap();
        let reasoning_seen = std::sync::Mutex::new(String::new());
        let content_seen = std::sync::Mutex::new(String::new());
        let r = provider
            .chat_stream(
                &request(),
                &|c| content_seen.lock().unwrap().push_str(c),
                &|r| reasoning_seen.lock().unwrap().push_str(r),
                CancellationToken::new(),
            )
            .await
            .expect("stream must succeed");
        assert_eq!(r.content, "Bonjour le monde");
        assert_eq!(r.reasoning.as_deref(), Some("Je réfléchis. "));
        assert_eq!(content_seen.lock().unwrap().as_str(), "Bonjour le monde");
        assert!(!r.truncated);
        let usage = r.usage.expect("final usage frame");
        assert_eq!((usage.prompt_tokens, usage.cached_tokens, usage.completion_tokens, usage.reasoning_tokens), (42, 40, 9, 4));
    }

    #[tokio::test]
    async fn connection_cut_mid_stream_keeps_partial_content_without_retry() {
        let base = serve_once(
            "200 OK",
            vec!["data: {\"choices\":[{\"delta\":{\"content\":\"Début de réponse\"},\"finish_reason\":null}]}\n\n"],
            true,
        )
        .await;
        let provider = DeepSeekProvider::with_base_url(&base, "sk-test", "deepseek-flash", 8192).unwrap();
        let started = std::time::Instant::now();
        let r = provider.chat_stream(&request(), &|_| {}, &|_| {}, CancellationToken::new()).await.expect("partial content is kept");
        assert_eq!(r.content, "Début de réponse");
        assert!(r.usage.is_none(), "no usage frame → estimated later by the caller");
        // Content was emitted → the provider must NOT retry (no backoff delay)
        assert!(started.elapsed() < Duration::from_millis(constants::DEEPSEEK_RETRY_BASE_MS), "no retry after emitted content");
    }

    #[tokio::test]
    async fn insufficient_balance_is_fatal_and_not_retried() {
        let base = serve_once("402 Payment Required", vec!["{\"error\":{\"message\":\"Insufficient Balance\"}}"], true).await;
        let provider = DeepSeekProvider::with_base_url(&base, "sk-test", "deepseek-flash", 8192).unwrap();
        let started = std::time::Instant::now();
        let err = provider.chat_stream(&request(), &|_| {}, &|_| {}, CancellationToken::new()).await.expect_err("must fail");
        assert!(matches!(err, LlmError::InsufficientBalance), "{err:?}");
        assert!(started.elapsed() < Duration::from_millis(constants::DEEPSEEK_RETRY_BASE_MS));
    }
}
