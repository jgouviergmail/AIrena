//! Usage-metering decorator: records every provider response's token usage
//! into a shared [`UsageLedger`], attributed by call kind and speaker.
//!
//! The engine, the turn manager and the RAG store all go through the same
//! provider handle, so metering is centralised here instead of at each call site.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use super::pricing;
use super::{LlmCapabilities, LlmError, LlmProvider, LlmRequest, LlmResponse, TokenCallback};
use crate::models::llm::{ProviderKind, UsageLedger};

pub struct MeteredProvider {
    inner: Arc<dyn LlmProvider>,
    ledger: Mutex<UsageLedger>,
    /// Set once a fatal provider error (auth, balance, unknown model) was observed.
    fatal: AtomicBool,
}

impl MeteredProvider {
    pub fn new(inner: Arc<dyn LlmProvider>) -> Self {
        Self { inner, ledger: Mutex::new(UsageLedger::default()), fatal: AtomicBool::new(false) }
    }

    /// A fatal error was returned by the provider — further calls are pointless.
    pub fn has_fatal_error(&self) -> bool {
        self.fatal.load(Ordering::Relaxed)
    }

    /// Snapshot of the ledger (recovers from a poisoned lock — the ledger is
    /// always left consistent).
    pub fn snapshot(&self) -> UsageLedger {
        self.ledger.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn record(&self, request: &LlmRequest, response: &LlmResponse) {
        let Some(usage) = &response.usage else { return };
        // Price each call at the tariff in force when it completed.
        let cost = if self.inner.capabilities().billable {
            pricing::estimate_cost_usd(self.inner.model_name(), usage, pricing::is_peak_hour(chrono::Utc::now()))
        } else {
            None
        };
        let mut ledger = self.ledger.lock().unwrap_or_else(|e| e.into_inner());
        ledger.record(request.call_kind, request.speaker_id.as_deref(), usage, cost);
    }
}

#[async_trait]
impl LlmProvider for MeteredProvider {
    fn kind(&self) -> ProviderKind {
        self.inner.kind()
    }

    fn model_name(&self) -> &str {
        self.inner.model_name()
    }

    fn capabilities(&self) -> &LlmCapabilities {
        self.inner.capabilities()
    }

    async fn chat_stream(
        &self,
        request: &LlmRequest,
        on_content: TokenCallback<'_>,
        on_reasoning: TokenCallback<'_>,
        cancel: CancellationToken,
    ) -> Result<LlmResponse, LlmError> {
        let response = match self.inner.chat_stream(request, on_content, on_reasoning, cancel).await {
            Ok(r) => r,
            Err(e) => {
                if e.is_fatal() {
                    self.fatal.store(true, Ordering::Relaxed);
                }
                return Err(e);
            }
        };
        self.record(request, &response);
        Ok(response)
    }

    async fn validate(&self) -> Result<(), LlmError> {
        self.inner.validate().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::mock::MockLlmProvider;
    use crate::models::llm::{CallKind, LlmUsage};
    use crate::models::settings::LlmParams;

    #[tokio::test]
    async fn records_usage_by_kind_and_speaker() {
        let mock = MockLlmProvider::scripted(|_req| {
            Ok(LlmResponse {
                content: "ok".into(),
                reasoning: None,
                usage: Some(LlmUsage { prompt_tokens: 10, cached_tokens: 2, completion_tokens: 5, reasoning_tokens: 1 }),
                truncated: false,
            })
        });
        let metered = MeteredProvider::new(Arc::new(mock));
        let params = LlmParams::default();

        metered.chat(&LlmRequest::new("s", "u", &params, CallKind::Intervention).speaker("g1"), CancellationToken::new()).await.unwrap();
        metered.chat(&LlmRequest::new("s", "u", &params, CallKind::Reaction).speaker("g1"), CancellationToken::new()).await.unwrap();
        metered.chat(&LlmRequest::new("s", "u", &params, CallKind::Memory), CancellationToken::new()).await.unwrap();

        let ledger = metered.snapshot();
        assert_eq!(ledger.calls, 3);
        assert_eq!(ledger.total.prompt_tokens, 30);
        assert_eq!(ledger.total.reasoning_tokens, 3);
        assert_eq!(ledger.by_speaker["g1"].completion_tokens, 10);
        assert_eq!(ledger.by_call_kind[&CallKind::Memory].prompt_tokens, 10);
        // Mock is not billable → no cost
        assert!(ledger.estimated_cost_usd.is_none());
    }

    #[tokio::test]
    async fn prices_calls_for_billable_providers() {
        let mock = MockLlmProvider::scripted(|_req| {
            Ok(LlmResponse {
                content: "ok".into(),
                reasoning: None,
                usage: Some(LlmUsage { prompt_tokens: 1_000_000, cached_tokens: 0, completion_tokens: 0, reasoning_tokens: 0 }),
                truncated: false,
            })
        })
        .with_capabilities(LlmCapabilities {
            supports_reasoning: true,
            reasoning_levels: true,
            reasoning_displayable: true,
            supports_json_mode: true,
            context_tokens: 1,
            chars_per_token_latin: 3.3,
            chars_per_token_cjk: 1.7,
            reports_usage: true,
            billable: true,
        })
        .with_model_name("deepseek-flash");
        let metered = MeteredProvider::new(Arc::new(mock));
        metered.chat(&LlmRequest::new("s", "u", &LlmParams::default(), CallKind::Memory), CancellationToken::new()).await.unwrap();
        let cost = metered.snapshot().estimated_cost_usd.unwrap();
        // 1M uncached input tokens: 0.30 (peak) or 0.15 (off-peak)
        assert!((cost - 0.30).abs() < 1e-9 || (cost - 0.15).abs() < 1e-9, "{cost}");
    }

    #[tokio::test]
    async fn ignores_responses_without_usage_and_propagates_errors() {
        let mock = MockLlmProvider::scripted(|req| match req.call_kind {
            CallKind::Moderation => Err(LlmError::RateLimited),
            _ => Ok(LlmResponse { content: "x".into(), ..Default::default() }),
        });
        let metered = MeteredProvider::new(Arc::new(mock));
        let params = LlmParams::default();

        metered.chat(&LlmRequest::new("s", "u", &params, CallKind::Vote), CancellationToken::new()).await.unwrap();
        let err = metered.chat(&LlmRequest::new("s", "u", &params, CallKind::Moderation), CancellationToken::new()).await.unwrap_err();
        assert!(matches!(err, LlmError::RateLimited));
        assert_eq!(metered.snapshot().calls, 0);
        assert!(!metered.has_fatal_error(), "rate limits are transient");
    }

    #[tokio::test]
    async fn fatal_errors_are_latched() {
        let mock = MockLlmProvider::scripted(|_| Err(LlmError::InsufficientBalance));
        let metered = MeteredProvider::new(Arc::new(mock));
        let _ = metered.chat(&LlmRequest::new("s", "u", &LlmParams::default(), CallKind::Vote), CancellationToken::new()).await;
        assert!(metered.has_fatal_error());
    }
}
