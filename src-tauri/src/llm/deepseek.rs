//! DeepSeek provider — the OpenAI-compatible transport configured for the
//! DeepSeek dialect, plus the DeepSeek-only endpoints (`/user/balance`).
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
//!
//! The wire mapping, the SSE parsing and the retry policy live in
//! [`super::openai_compat`] (shared with the generic OpenAI-compatible provider).

use std::fmt;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use super::openai_compat::{Dialect, OpenAiCompatTransport};
#[cfg(test)]
use super::openai_compat::ChatCompletionRequest;
use super::{LlmCapabilities, LlmError, LlmProvider, LlmRequest, LlmResponse, TokenCallback};
use crate::constants;
use crate::models::llm::ProviderKind;
#[cfg(test)]
use super::openai_compat::{backoff_delay, parse_sse_line, SseLine, StreamChunk, WireUsage};
#[cfg(test)]
use crate::models::llm::{LlmUsage, ReasoningLevel, ReasoningPace};
#[cfg(test)]
use std::time::Duration;

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
    transport: OpenAiCompatTransport,
    caps: LlmCapabilities,
}

impl fmt::Debug for DeepSeekProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeepSeekProvider")
            .field("base_url", &constants::DEEPSEEK_BASE_URL)
            .field("model", &self.transport.model())
            .field("api_key", &"***")
            .finish()
    }
}

impl DeepSeekProvider {
    pub fn new(api_key: &str, model: &str, context_budget: u32) -> Result<Self, LlmError> {
        Self::with_base_url(constants::DEEPSEEK_BASE_URL, api_key, model, context_budget)
    }

    /// Constructor with a custom base URL (tests against a local mock server).
    pub fn with_base_url(base_url: &str, api_key: &str, model: &str, context_budget: u32) -> Result<Self, LlmError> {
        if api_key.trim().is_empty() {
            return Err(LlmError::Auth);
        }
        let model = if model.trim().is_empty() { constants::DEEPSEEK_DEFAULT_MODEL } else { model.trim() };
        let transport = OpenAiCompatTransport::new(base_url, api_key, model, Dialect::DeepSeek)?;
        Ok(Self {
            transport,
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
                max_parallel_calls: constants::DEEPSEEK_MAX_PARALLEL_CALLS,
            },
        })
    }

    /// Translate a generic request into the chat-completions body (DeepSeek dialect).
    #[cfg(test)]
    pub fn to_wire(&self, request: &LlmRequest) -> ChatCompletionRequest {
        self.transport.to_wire(request)
    }

    /// Map an HTTP error response to `LlmError` (shared classification).
    #[cfg(test)]
    fn classify_http_error(status: u16, body: &str) -> LlmError {
        super::openai_compat::classify_http_error(status, body)
    }

    /// `GET /models` → model ids. Falls back to the documented list when the API lists none.
    pub async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        let mut ids = self.transport.list_models().await?;
        if ids.is_empty() {
            ids = constants::DEEPSEEK_KNOWN_MODELS.iter().map(|s| s.to_string()).collect();
        }
        Ok(ids)
    }

    /// `GET /user/balance` — also the cheapest way to validate an API key.
    pub async fn balance(&self) -> Result<DeepSeekBalance, LlmError> {
        let resp = self
            .transport
            .get(constants::DEEPSEEK_BALANCE_PATH)
            .send()
            .await
            .map_err(|e| LlmError::Connection(e.to_string()))?;
        if resp.status().as_u16() != 200 {
            return Err(OpenAiCompatTransport::error_from(resp).await);
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

#[async_trait]
impl LlmProvider for DeepSeekProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::DeepSeek
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

    async fn validate(&self) -> Result<(), LlmError> {
        let balance = self.balance().await?;
        if !balance.is_available {
            return Err(LlmError::InsufficientBalance);
        }
        let models = self.list_models().await?;
        if !models.iter().any(|m| m == self.model_name()) {
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
        // Fast pace: the allowance is scaled down (S12), the level itself is untouched
        let params = LlmParams { num_predict: 1000, ..Default::default() };
        let req = LlmRequest::new("s", "u", &params, CallKind::Intervention).reasoning(ReasoningLevel::High).pace(ReasoningPace::Fast);
        let json = serde_json::to_value(provider().to_wire(&req)).unwrap();
        let expected = (f64::from(constants::DEEPSEEK_REASONING_ALLOWANCE_HIGH) * constants::DEEPSEEK_FAST_PACE_ALLOWANCE_FACTOR) as i64;
        assert_eq!(json["max_tokens"], 1000 + expected);
        assert_eq!(json["reasoning_effort"], "high");
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
