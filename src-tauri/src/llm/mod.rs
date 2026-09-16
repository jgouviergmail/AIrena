//! Provider-agnostic LLM abstraction.
//!
//! The discussion engine, the turn manager and the RAG store depend only on the
//! [`LlmProvider`] trait. Concrete providers translate the generic
//! [`LlmRequest`] into their wire format and normalise responses, errors and
//! token usage:
//!
//! - [`ollama::OllamaProvider`] — local Ollama (iso-functional with the historical client)
//! - [`deepseek::DeepSeekProvider`] — DeepSeek cloud API (OpenAI-compatible, SSE)
//! - [`metered::MeteredProvider`] — decorator recording usage into a [`UsageLedger`]
//!
//! Streaming is the only transport: `chat()` is `chat_stream()` with no-op callbacks.

pub mod deepseek;
pub mod factory;
pub mod metered;
pub mod ollama;
pub mod pricing;
#[cfg(test)]
pub mod mock;

use async_trait::async_trait;
use serde::Serialize;
use tokio_util::sync::CancellationToken;

use crate::models::llm::{CallKind, LlmUsage, ProviderKind, ReasoningLevel};
use crate::models::settings::LlmParams;
use crate::ollama::error::OllamaError;

/// Streaming token callback (object-safe, shared across await points).
pub type TokenCallback<'a> = &'a (dyn Fn(&str) + Send + Sync);

/// Generic chat request built by the engine. Providers own the wire mapping.
#[derive(Debug, Clone)]
pub struct LlmRequest {
    pub system: String,
    pub user: String,
    pub params: LlmParams,
    /// Ask the provider for a JSON object (Ollama `format: json`, DeepSeek `json_object`).
    pub json_mode: bool,
    /// Resolved reasoning level — never [`ReasoningLevel::Auto`].
    pub reasoning: ReasoningLevel,
    pub call_kind: CallKind,
    /// Speaker on whose behalf the call is made (usage attribution).
    pub speaker_id: Option<String>,
}

impl LlmRequest {
    pub fn new(system: &str, user: &str, params: &LlmParams, call_kind: CallKind) -> Self {
        Self {
            system: system.to_string(),
            user: user.to_string(),
            params: params.clone(),
            json_mode: false,
            reasoning: ReasoningLevel::Off,
            call_kind,
            speaker_id: None,
        }
    }

    /// Structured JSON output: enables the provider's JSON mode and pins the
    /// low `TEMP_JSON_OUTPUT` temperature (every utility call shares it, so
    /// that a persona's creative temperature never leaks into parsers).
    pub fn json(mut self) -> Self {
        self.json_mode = true;
        self.params.temperature = crate::constants::TEMP_JSON_OUTPUT;
        self
    }

    pub fn reasoning(mut self, level: ReasoningLevel) -> Self {
        debug_assert!(level != ReasoningLevel::Auto, "Auto must be resolved by the engine");
        self.reasoning = if level == ReasoningLevel::Auto { ReasoningLevel::Off } else { level };
        self
    }

    pub fn speaker(mut self, speaker_id: &str) -> Self {
        self.speaker_id = Some(speaker_id.to_string());
        self
    }
}

/// Normalised provider response.
#[derive(Debug, Clone, Default)]
pub struct LlmResponse {
    /// Final answer content (never contains leaked `<think>` blocks).
    pub content: String,
    /// Reasoning text when the provider exposes it.
    pub reasoning: Option<String>,
    /// Token usage when the provider reports it.
    pub usage: Option<LlmUsage>,
    /// The generation stopped because of the output token limit.
    pub truncated: bool,
}

/// Static description of what a provider/model combination can do.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmCapabilities {
    /// The model can reason before answering.
    pub supports_reasoning: bool,
    /// `Low`/`High`/`Max` map to distinct provider behaviours (DeepSeek); for Ollama
    /// any active level simply means "let the model think".
    pub reasoning_levels: bool,
    /// The reasoning text is meaningful enough to be shown to users.
    pub reasoning_displayable: bool,
    pub supports_json_mode: bool,
    /// Context window (tokens) the engine may rely on.
    pub context_tokens: u32,
    pub chars_per_token_latin: f64,
    pub chars_per_token_cjk: f64,
    /// Provider returns token counts.
    pub reports_usage: bool,
    /// Usage costs money.
    pub billable: bool,
}

/// Unified error type. Providers map their transport/API errors onto it.
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("Request cancelled")]
    Cancelled,
    #[error("Connection failed: {0}")]
    Connection(String),
    #[error("Authentication failed: invalid API key")]
    Auth,
    #[error("Insufficient balance on the provider account")]
    InsufficientBalance,
    #[error("Rate limited by the provider")]
    RateLimited,
    #[error("Provider overloaded")]
    Overloaded,
    #[error("Client error: {0}")]
    Client(String),
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    #[error("JSON parse error: {0}")]
    Json(String),
}

impl LlmError {
    /// Transient failures worth retrying with backoff.
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Connection(_) | Self::RateLimited | Self::Overloaded)
    }

    /// Failures that should end the discussion (no point in retrying any call).
    pub fn is_fatal(&self) -> bool {
        matches!(self, Self::Auth | Self::InsufficientBalance | Self::ModelNotFound(_))
    }
}

impl From<OllamaError> for LlmError {
    fn from(e: OllamaError) -> Self {
        match e {
            OllamaError::Cancelled => Self::Cancelled,
            OllamaError::ConnectionFailed(m) => Self::Connection(m),
            OllamaError::ConnectionLost => Self::Connection("Connection lost after retries".to_string()),
            OllamaError::ClientError(m) => Self::Client(m),
            OllamaError::ModelNotFound(m) => Self::ModelNotFound(m),
            OllamaError::RequestFailed(e) => Self::Connection(e.to_string()),
            OllamaError::JsonError(e) => Self::Json(e.to_string()),
        }
    }
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn kind(&self) -> ProviderKind;
    fn model_name(&self) -> &str;
    fn capabilities(&self) -> &LlmCapabilities;

    /// Stream a chat completion. Content tokens go to `on_content`, reasoning
    /// tokens (when exposed) to `on_reasoning`. Returns the full response.
    async fn chat_stream(
        &self,
        request: &LlmRequest,
        on_content: TokenCallback<'_>,
        on_reasoning: TokenCallback<'_>,
        cancel: CancellationToken,
    ) -> Result<LlmResponse, LlmError>;

    /// Non-streaming convenience (JSON utilities): same transport, no callbacks.
    async fn chat(&self, request: &LlmRequest, cancel: CancellationToken) -> Result<LlmResponse, LlmError> {
        self.chat_stream(request, &|_| {}, &|_| {}, cancel).await
    }

    /// Pre-flight check: credentials valid and model available.
    async fn validate(&self) -> Result<(), LlmError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_builder_defaults_and_chaining() {
        let params = LlmParams::default();
        let req = LlmRequest::new("sys", "usr", &params, CallKind::Reaction);
        assert!(!req.json_mode);
        assert_eq!(req.reasoning, ReasoningLevel::Off);
        assert!(req.speaker_id.is_none());

        let req = req.json().reasoning(ReasoningLevel::High).speaker("g1");
        assert!(req.json_mode);
        assert_eq!(req.reasoning, ReasoningLevel::High);
        assert_eq!(req.speaker_id.as_deref(), Some("g1"));
    }

    #[test]
    fn error_classification() {
        assert!(LlmError::RateLimited.is_retryable());
        assert!(LlmError::Overloaded.is_retryable());
        assert!(LlmError::Connection("x".into()).is_retryable());
        assert!(!LlmError::Auth.is_retryable());
        assert!(LlmError::Auth.is_fatal());
        assert!(LlmError::InsufficientBalance.is_fatal());
        assert!(!LlmError::Cancelled.is_fatal());
        assert!(!LlmError::Client("bad".into()).is_retryable());
    }

    #[test]
    fn ollama_error_mapping() {
        assert!(matches!(LlmError::from(OllamaError::Cancelled), LlmError::Cancelled));
        assert!(matches!(LlmError::from(OllamaError::ConnectionLost), LlmError::Connection(_)));
        assert!(matches!(LlmError::from(OllamaError::ClientError("HTTP 400".into())), LlmError::Client(_)));
        assert!(matches!(LlmError::from(OllamaError::ModelNotFound("m".into())), LlmError::ModelNotFound(_)));
    }
}
