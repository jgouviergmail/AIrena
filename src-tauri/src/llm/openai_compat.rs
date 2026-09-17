//! OpenAI-compatible chat-completions transport over SSE (v1.20).
//!
//! One transport, two dialects:
//! - [`Dialect::DeepSeek`] — the DeepSeek API (`thinking`, `reasoning_effort`,
//!   reasoning allowances, `top_p` floor while thinking, `reasoning_content` deltas);
//! - [`Dialect::Generic`] — any OpenAI-compatible server (LM Studio, vLLM,
//!   llama.cpp, OpenRouter…): plain sampling, no reasoning fields, an empty API
//!   key allowed for local servers.
//!
//! The wire types, the SSE parsing, the retry/backoff policy and the usage
//! normalisation are shared; [`super::deepseek::DeepSeekProvider`] configures
//! the transport, [`OpenAiCompatProvider`] is the generic provider.

use std::fmt;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use super::{LlmCapabilities, LlmError, LlmProvider, LlmRequest, LlmResponse, TokenCallback};
use crate::constants;
use crate::models::llm::{LlmUsage, ProviderKind, ReasoningLevel, ReasoningPace};

/// Which flavour of the OpenAI-compatible API the server speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    DeepSeek,
    Generic,
}

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
    /// DeepSeek dialect only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<Thinking>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<&'static str>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct Delta {
    #[serde(default)]
    pub(crate) content: Option<String>,
    #[serde(default)]
    pub(crate) reasoning_content: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct StreamChoice {
    #[serde(default)]
    pub(crate) delta: Delta,
    #[serde(default)]
    pub(crate) finish_reason: Option<String>,
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
pub(crate) struct WireUsage {
    #[serde(default)]
    pub(crate) prompt_tokens: u32,
    #[serde(default)]
    pub(crate) completion_tokens: u32,
    #[serde(default)]
    prompt_tokens_details: Option<PromptTokensDetails>,
    #[serde(default)]
    completion_tokens_details: Option<CompletionTokensDetails>,
    /// Legacy top-level field kept by the DeepSeek API for compatibility.
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
pub(crate) struct StreamChunk {
    #[serde(default)]
    pub(crate) choices: Vec<StreamChoice>,
    #[serde(default)]
    pub(crate) usage: Option<WireUsage>,
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

// ── Transport ───────────────────────────────────────────────────────────

/// Outcome of one streaming attempt: the error plus whether tokens were already
/// forwarded (a retry would duplicate them).
type AttemptError = (LlmError, bool);

pub struct OpenAiCompatTransport {
    http: reqwest::Client,
    base_url: String,
    /// Empty for a local server without authentication
    api_key: String,
    model: String,
    dialect: Dialect,
}

impl fmt::Debug for OpenAiCompatTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenAiCompatTransport")
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("dialect", &self.dialect)
            .field("api_key", &if self.api_key.is_empty() { "(none)" } else { "***" })
            .finish()
    }
}

impl OpenAiCompatTransport {
    /// `base_url` without the trailing slash; `api_key` may be empty (generic dialect).
    pub fn new(base_url: &str, api_key: &str, model: &str, dialect: Dialect) -> Result<Self, LlmError> {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(constants::DEEPSEEK_CONNECT_TIMEOUT_SECS))
            .build()
            .map_err(|e| LlmError::Connection(format!("HTTP client build failed: {e}")))?;
        Ok(Self {
            http,
            base_url: base_url.trim().trim_end_matches('/').to_string(),
            api_key: api_key.trim().to_string(),
            model: model.trim().to_string(),
            dialect,
        })
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Authenticated GET (the bearer header only when a key is set).
    pub fn get(&self, path: &str) -> reqwest::RequestBuilder {
        let req = self.http.get(self.url(path));
        if self.api_key.is_empty() { req } else { req.bearer_auth(&self.api_key) }
    }

    fn post(&self, path: &str) -> reqwest::RequestBuilder {
        let req = self.http.post(self.url(path));
        if self.api_key.is_empty() { req } else { req.bearer_auth(&self.api_key) }
    }

    /// Read the HTTP error body of a non-200 answer and classify it.
    pub async fn error_from(response: reqwest::Response) -> LlmError {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        classify_http_error(status, &body)
    }

    fn effort_name(level: ReasoningLevel) -> Option<&'static str> {
        match level {
            ReasoningLevel::Low => Some("low"),
            ReasoningLevel::High => Some("high"),
            ReasoningLevel::Max => Some("max"),
            ReasoningLevel::Off | ReasoningLevel::Auto => None,
        }
    }

    fn reasoning_allowance(level: ReasoningLevel, pace: ReasoningPace) -> i32 {
        let base = match level {
            ReasoningLevel::Low => constants::DEEPSEEK_REASONING_ALLOWANCE_LOW,
            ReasoningLevel::High => constants::DEEPSEEK_REASONING_ALLOWANCE_HIGH,
            ReasoningLevel::Max => constants::DEEPSEEK_REASONING_ALLOWANCE_MAX,
            ReasoningLevel::Off | ReasoningLevel::Auto => 0,
        };
        match pace {
            ReasoningPace::Normal => base,
            ReasoningPace::Fast => (f64::from(base) * constants::DEEPSEEK_FAST_PACE_ALLOWANCE_FACTOR) as i32,
        }
    }

    /// Translate a generic request into the chat-completions body of the dialect.
    pub fn to_wire(&self, request: &LlmRequest) -> ChatCompletionRequest {
        let params = &request.params;
        let mut messages = Vec::with_capacity(2);
        if !request.system.is_empty() {
            messages.push(WireMessage { role: "system", content: request.system.clone() });
        }
        messages.push(WireMessage { role: "user", content: request.user.clone() });
        let response_format = request.json_mode.then_some(ResponseFormat { kind: "json_object" });

        match self.dialect {
            Dialect::DeepSeek => {
                let thinking = request.reasoning.is_active();
                let max_tokens = params
                    .num_predict
                    .saturating_add(Self::reasoning_allowance(request.reasoning, request.pace))
                    .clamp(1, constants::DEEPSEEK_MAX_OUTPUT_TOKENS);
                ChatCompletionRequest {
                    model: self.model.clone(),
                    messages,
                    stream: true,
                    stream_options: StreamOptions { include_usage: true },
                    max_tokens,
                    // Unsupported while thinking — omitted rather than rejected by the API.
                    temperature: (!thinking).then_some(params.temperature),
                    top_p: Some(if thinking { params.top_p.max(constants::DEEPSEEK_TOP_P_MIN_THINKING) } else { params.top_p }),
                    response_format,
                    thinking: Some(Thinking { kind: if thinking { "enabled" } else { "disabled" } }),
                    reasoning_effort: if thinking { Self::effort_name(request.reasoning) } else { None },
                }
            }
            // Generic servers know nothing of reasoning fields: never send them (S39)
            Dialect::Generic => ChatCompletionRequest {
                model: self.model.clone(),
                messages,
                stream: true,
                stream_options: StreamOptions { include_usage: true },
                max_tokens: params.num_predict.clamp(1, constants::OPENAI_COMPAT_MAX_OUTPUT_TOKENS),
                temperature: Some(params.temperature),
                top_p: Some(params.top_p),
                response_format,
                thinking: None,
                reasoning_effort: None,
            },
        }
    }

    async fn stream_once(
        &self,
        wire: &ChatCompletionRequest,
        on_content: TokenCallback<'_>,
        on_reasoning: TokenCallback<'_>,
        cancel: &CancellationToken,
    ) -> Result<LlmResponse, AttemptError> {
        let send = self.post(constants::DEEPSEEK_CHAT_PATH).json(wire).send();
        let response = tokio::select! {
            r = send => r.map_err(|e| (LlmError::Connection(e.to_string()), false))?,
            _ = cancel.cancelled() => return Err((LlmError::Cancelled, false)),
        };
        if response.status().as_u16() != 200 {
            return Err((Self::error_from(response).await, false));
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
            tracing::warn!(chars = content.len(), model = %self.model, "Response truncated (max_tokens reached)");
        }
        Ok(LlmResponse {
            content: content.trim().to_string(),
            reasoning: (!reasoning.is_empty()).then_some(reasoning),
            usage,
            truncated,
        })
    }

    /// Stream with the shared retry policy: transient errors are retried with
    /// backoff only while nothing was emitted yet.
    pub async fn chat_stream(
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
                    tracing::warn!(error = %e, attempt = attempt + 1, delay_ms = delay.as_millis() as u64, dialect = ?self.dialect, "Transient error — retrying");
                    tokio::select! {
                        _ = tokio::time::sleep(delay) => {}
                        _ = cancel.cancelled() => return Err(LlmError::Cancelled),
                    }
                    attempt += 1;
                }
            }
        }
    }

    /// `GET /models` → model ids (empty when the server lists none).
    pub async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        let resp = self
            .get(constants::DEEPSEEK_MODELS_PATH)
            .send()
            .await
            .map_err(|e| LlmError::Connection(e.to_string()))?;
        if resp.status().as_u16() != 200 {
            return Err(Self::error_from(resp).await);
        }
        let parsed: ModelsResponse = resp.json().await.map_err(|e| LlmError::Json(format!("models response: {e}")))?;
        Ok(parsed.data.into_iter().map(|m| m.id).collect())
    }
}

/// Map an HTTP error response to `LlmError` (body is the API's JSON error when it has one).
pub(crate) fn classify_http_error(status: u16, body: &str) -> LlmError {
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

pub(crate) enum SseLine<'a> {
    Skip,
    Done,
    Data(&'a str),
}

/// Parse one SSE line: comments (`: keep-alive`), empty lines and non-data
/// fields are skipped; `data: [DONE]` ends the stream.
pub(crate) fn parse_sse_line(line: &str) -> SseLine<'_> {
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

pub(crate) fn backoff_delay(attempt: u32) -> Duration {
    use rand::Rng;
    let base = constants::DEEPSEEK_RETRY_BASE_MS.saturating_mul(1u64 << attempt.min(6));
    let jitter = rand::thread_rng().gen_range(0..=constants::DEEPSEEK_RETRY_JITTER_MS);
    Duration::from_millis(base + jitter)
}

// ── Generic provider ────────────────────────────────────────────────────

/// Any OpenAI-compatible server: no reasoning, usage when reported, never billed.
#[derive(Debug)]
pub struct OpenAiCompatProvider {
    transport: OpenAiCompatTransport,
    caps: LlmCapabilities,
}

impl OpenAiCompatProvider {
    pub fn new(base_url: &str, api_key: &str, model: &str, context_tokens: u32) -> Result<Self, LlmError> {
        if base_url.trim().is_empty() {
            return Err(LlmError::Connection("No OpenAI-compatible base URL configured".to_string()));
        }
        if model.trim().is_empty() {
            return Err(LlmError::ModelNotFound("No OpenAI-compatible model configured".to_string()));
        }
        let transport = OpenAiCompatTransport::new(base_url, api_key, model, Dialect::Generic)?;
        Ok(Self {
            transport,
            caps: LlmCapabilities {
                supports_reasoning: false,
                reasoning_levels: false,
                reasoning_displayable: false,
                supports_json_mode: true,
                context_tokens,
                chars_per_token_latin: constants::CHARS_PER_TOKEN_LATIN,
                chars_per_token_cjk: constants::CHARS_PER_TOKEN_CJK,
                reports_usage: true,
                billable: false,
                max_parallel_calls: constants::OPENAI_COMPAT_MAX_PARALLEL_CALLS,
            },
        })
    }

    /// The server's model list (`GET /models`). A server without catalogue
    /// (HTTP 4xx, unreadable payload — many local servers) yields an empty list;
    /// an unreachable or failing server, or a refused key, is an error.
    pub async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        match self.transport.list_models().await {
            Ok(models) => Ok(models),
            Err(e @ (LlmError::Client(_) | LlmError::Json(_))) => {
                tracing::warn!(error = %e, "OpenAI-compatible server has no model catalogue — manual list applies");
                Ok(Vec::new())
            }
            Err(e) => Err(e),
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAiCompatProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::OpenAiCompat
    }

    fn model_name(&self) -> &str {
        self.transport.model()
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
        self.transport.chat_stream(request, on_content, on_reasoning, cancel).await
    }

    /// The key is accepted and, when the server publishes a catalogue, the model is in it.
    async fn validate(&self) -> Result<(), LlmError> {
        let models = self.list_models().await?;
        if !models.is_empty() && !models.iter().any(|m| m == self.model_name()) {
            return Err(LlmError::ModelNotFound(self.model_name().to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::llm::CallKind;
    use crate::models::settings::LlmParams;

    fn generic() -> OpenAiCompatProvider {
        OpenAiCompatProvider::new("http://localhost:1234/v1/", "", "local-model", 8192).unwrap()
    }

    #[test]
    fn generic_provider_accepts_an_empty_key_and_never_bills() {
        let p = generic();
        assert_eq!(p.kind(), ProviderKind::OpenAiCompat);
        assert_eq!(p.model_name(), "local-model");
        assert!(!p.capabilities().billable && !p.capabilities().supports_reasoning && p.capabilities().supports_json_mode);
        assert_eq!(p.capabilities().context_tokens, 8192);
        assert!(matches!(OpenAiCompatProvider::new("", "", "m", 1).unwrap_err(), LlmError::Connection(_)));
        assert!(matches!(OpenAiCompatProvider::new("http://x", "", " ", 1).unwrap_err(), LlmError::ModelNotFound(_)));
        let dbg = format!("{:?}", OpenAiCompatProvider::new("http://x", "sk-secret", "m", 1).unwrap());
        assert!(!dbg.contains("sk-secret") && dbg.contains("***"));
        assert!(format!("{:?}", generic()).contains("(none)"));
    }

    /// S39 — the generic dialect never sends the DeepSeek reasoning fields, even
    /// when the engine asked for reasoning; sampling is passed through.
    #[test]
    fn generic_wire_has_no_reasoning_fields_and_keeps_sampling() {
        let params = LlmParams { temperature: 0.8, top_p: 0.6, num_predict: 700, ..Default::default() };
        let req = LlmRequest::new("SYS", "USER", &params, CallKind::Intervention).reasoning(ReasoningLevel::High);
        let json = serde_json::to_value(generic().transport.to_wire(&req)).unwrap();
        assert_eq!(json["model"], "local-model");
        assert!(json.get("thinking").is_none() && json.get("reasoning_effort").is_none());
        assert_eq!(json["temperature"], 0.8f32);
        assert_eq!(json["top_p"], 0.6f32);
        assert_eq!(json["max_tokens"], 700);
        assert!(json.get("response_format").is_none());
        assert_eq!(json["stream"], true);
        let req = LlmRequest::new("SYS", "USER", &params, CallKind::Reaction).json();
        let json = serde_json::to_value(generic().transport.to_wire(&req)).unwrap();
        assert_eq!(json["response_format"]["type"], "json_object");
        assert_eq!(json["temperature"], constants::TEMP_JSON_OUTPUT);
        // Bounded output
        let params = LlmParams { num_predict: i32::MAX, ..Default::default() };
        assert_eq!(generic().transport.to_wire(&LlmRequest::new("s", "u", &params, CallKind::Vote)).max_tokens, constants::OPENAI_COMPAT_MAX_OUTPUT_TOKENS);
    }

    #[test]
    fn transport_normalises_the_base_url() {
        let t = OpenAiCompatTransport::new("  https://api.example.com/v1/ ", " key ", " m ", Dialect::Generic).unwrap();
        assert_eq!(t.url("/models"), "https://api.example.com/v1/models");
        assert_eq!(t.model(), "m");
        assert_eq!(t.dialect, Dialect::Generic);
    }
}

/// Throw-away local HTTP server for the transport and factory tests.
#[cfg(test)]
pub(crate) mod test_support {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    /// Serve one response for the next connection and hand back the request head seen.
    pub(crate) async fn serve_once(status: &'static str, body: &'static str) -> (String, std::sync::Arc<std::sync::Mutex<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let seen = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let seen_w = std::sync::Arc::clone(&seen);
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 16 * 1024];
            let n = socket.read(&mut buf).await.unwrap_or(0);
            *seen_w.lock().unwrap() = String::from_utf8_lossy(&buf[..n]).to_string();
            let head = format!("HTTP/1.1 {status}\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n");
            socket.write_all(head.as_bytes()).await.unwrap();
            socket.write_all(body.as_bytes()).await.unwrap();
            socket.flush().await.unwrap();
        });
        (format!("http://{addr}"), seen)
    }
}

/// Transport tests against a throw-away local HTTP server for the generic
/// dialect: an unauthenticated local server streams content; a 401 is fatal.
#[cfg(test)]
mod transport_tests {
    use super::test_support::serve_once;
    use super::*;
    use crate::models::llm::CallKind;
    use crate::models::settings::LlmParams;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn local_server_without_key_streams_content_and_usage() {
        let (base, seen) = serve_once(
            "200 OK",
            "data: {\"choices\":[{\"delta\":{\"content\":\"Salut \"},\"finish_reason\":null}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"toi\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":5,\"completion_tokens\":2}}\n\ndata: [DONE]\n\n",
        )
        .await;
        let p = OpenAiCompatProvider::new(&base, "", "local-model", 4096).unwrap();
        let req = LlmRequest::new("SYS", "USER", &LlmParams::default(), CallKind::Intervention).reasoning(ReasoningLevel::High);
        let r = p.chat_stream(&req, &|_| {}, &|_| {}, CancellationToken::new()).await.expect("stream");
        assert_eq!(r.content, "Salut toi");
        assert_eq!(r.usage.map(|u| (u.prompt_tokens, u.completion_tokens)), Some((5, 2)));
        let head = seen.lock().unwrap().clone();
        assert!(!head.to_lowercase().contains("authorization:"), "no bearer header without a key: {head}");
        assert!(!head.contains("\"thinking\"") && !head.contains("reasoning_effort"), "{head}");
    }

    #[tokio::test]
    async fn unauthorized_is_fatal_and_not_retried() {
        let (base, _) = serve_once("401 Unauthorized", "{\"error\":{\"message\":\"Invalid API key\"}}").await;
        let p = OpenAiCompatProvider::new(&base, "bad-key", "m", 4096).unwrap();
        let started = std::time::Instant::now();
        let err = p.chat(&LlmRequest::new("s", "u", &LlmParams::default(), CallKind::Vote), CancellationToken::new()).await.expect_err("401");
        assert!(matches!(err, LlmError::Auth), "{err:?}");
        assert!(err.is_fatal());
        assert!(started.elapsed() < Duration::from_millis(constants::DEEPSEEK_RETRY_BASE_MS));
    }

    /// A server that does not answer at all must fail the validation (the
    /// "Test connection" button and `start_discussion` rely on it): only a
    /// missing catalogue (HTTP 404…) is tolerated.
    #[tokio::test]
    async fn validate_reports_an_unreachable_server() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let p = OpenAiCompatProvider::new(&format!("http://{addr}"), "", "m", 4096).unwrap();
        assert!(matches!(p.validate().await.unwrap_err(), LlmError::Connection(_)));
        assert!(matches!(p.list_models().await.unwrap_err(), LlmError::Connection(_)));
    }

    #[tokio::test]
    async fn validate_tolerates_a_missing_catalogue_but_not_an_absent_model() {
        // No /models endpoint (404): accepted
        let (base, _) = serve_once("404 Not Found", "{}").await;
        let p = OpenAiCompatProvider::new(&base, "", "m", 4096).unwrap();
        p.validate().await.expect("no catalogue → accepted");
        // A catalogue without the model: refused
        let (base, _) = serve_once("200 OK", "{\"data\":[{\"id\":\"other\"}]}").await;
        let p = OpenAiCompatProvider::new(&base, "", "m", 4096).unwrap();
        assert!(matches!(p.validate().await.unwrap_err(), LlmError::ModelNotFound(_)));
        // A catalogue with the model: accepted
        let (base, _) = serve_once("200 OK", "{\"data\":[{\"id\":\"m\"}]}").await;
        let p = OpenAiCompatProvider::new(&base, "", "m", 4096).unwrap();
        p.validate().await.expect("listed");
    }
}
