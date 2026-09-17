//! One model per speaker (v1.20): a [`RoutingProvider`] dispatches each request
//! to the provider of its speaker and falls back to the default provider
//! (the global model) for everybody else — utilities, the moderator, speakers
//! without an override.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use super::{LlmCapabilities, LlmError, LlmProvider, LlmRequest, LlmResponse, TokenCallback};
use crate::models::llm::ProviderKind;

pub struct RoutingProvider {
    default: Arc<dyn LlmProvider>,
    by_speaker: HashMap<String, Arc<dyn LlmProvider>>,
}

impl RoutingProvider {
    pub fn new(default: Arc<dyn LlmProvider>, by_speaker: HashMap<String, Arc<dyn LlmProvider>>) -> Self {
        Self { default, by_speaker }
    }

    /// The provider serving a speaker (the default one when the speaker has no override).
    pub fn provider_for(&self, speaker_id: Option<&str>) -> &Arc<dyn LlmProvider> {
        speaker_id.and_then(|id| self.by_speaker.get(id)).unwrap_or(&self.default)
    }

}

#[async_trait]
impl LlmProvider for RoutingProvider {
    fn kind(&self) -> ProviderKind {
        self.default.kind()
    }

    fn model_name(&self) -> &str {
        self.default.model_name()
    }

    fn capabilities(&self) -> &LlmCapabilities {
        self.default.capabilities()
    }

    fn model_for(&self, request: &LlmRequest) -> &str {
        self.provider_for(request.speaker_id.as_deref()).model_name()
    }

    fn capabilities_for(&self, speaker_id: Option<&str>) -> &LlmCapabilities {
        self.provider_for(speaker_id).capabilities()
    }

    async fn chat_stream(
        &self,
        request: &LlmRequest,
        on_content: TokenCallback<'_>,
        on_reasoning: TokenCallback<'_>,
        cancel: CancellationToken,
    ) -> Result<LlmResponse, LlmError> {
        self.provider_for(request.speaker_id.as_deref()).chat_stream(request, on_content, on_reasoning, cancel).await
    }

    /// Every distinct provider must pass (the default one first; a shared
    /// provider is validated once).
    async fn validate(&self) -> Result<(), LlmError> {
        self.default.validate().await?;
        let mut ids: Vec<&String> = self.by_speaker.keys().collect();
        ids.sort();
        let mut seen: Vec<&Arc<dyn LlmProvider>> = vec![&self.default];
        for id in ids {
            let p = &self.by_speaker[id];
            if seen.iter().any(|s| Arc::ptr_eq(s, p)) {
                continue;
            }
            seen.push(p);
            p.validate().await.map_err(|e| match e {
                LlmError::ModelNotFound(m) => LlmError::ModelNotFound(format!("{id}: {m}")),
                other => other,
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::mock::MockLlmProvider;
    use crate::models::llm::CallKind;
    use crate::models::settings::LlmParams;

    fn named(model: &str, reasoning: bool) -> Arc<dyn LlmProvider> {
        let model_owned = model.to_string();
        let mut caps = MockLlmProvider::scripted(|_| Ok(LlmResponse::default())).capabilities().clone();
        caps.supports_reasoning = reasoning;
        Arc::new(
            MockLlmProvider::scripted(move |_| Ok(LlmResponse { content: model_owned.clone(), ..Default::default() }))
                .with_capabilities(caps)
                .with_model_name(model),
        )
    }

    /// S23 — requests reach the speaker's model; utilities and unknown speakers use the default.
    #[tokio::test]
    async fn dispatches_by_speaker_and_reports_per_speaker_capabilities() {
        let a = named("model-a", true);
        let b = named("model-b", false);
        let router = RoutingProvider::new(Arc::clone(&a), HashMap::from([("g2".to_string(), Arc::clone(&b))]));
        let params = LlmParams::default();
        let r = router.chat(&LlmRequest::new("s", "u", &params, CallKind::Intervention).speaker("g2"), CancellationToken::new()).await.unwrap();
        assert_eq!(r.content, "model-b");
        let r = router.chat(&LlmRequest::new("s", "u", &params, CallKind::Intervention).speaker("g1"), CancellationToken::new()).await.unwrap();
        assert_eq!(r.content, "model-a");
        let r = router.chat(&LlmRequest::new("s", "u", &params, CallKind::Memory), CancellationToken::new()).await.unwrap();
        assert_eq!(r.content, "model-a");
        assert_eq!(router.model_for(&LlmRequest::new("s", "u", &params, CallKind::Reaction).speaker("g2")), "model-b");
        assert_eq!(router.model_for(&LlmRequest::new("s", "u", &params, CallKind::Reaction)), "model-a");
        assert!(router.capabilities_for(Some("g1")).supports_reasoning);
        assert!(!router.capabilities_for(Some("g2")).supports_reasoning);
        assert!(router.capabilities().supports_reasoning);
        assert_eq!(router.model_name(), "model-a");
        router.validate().await.unwrap();
    }

    /// S24 — an unusable speaker model is refused by the validation, naming the speaker.
    #[tokio::test]
    async fn validation_names_the_speaker_whose_model_is_missing() {
        let a = named("model-a", true);
        let missing: Arc<dyn LlmProvider> = Arc::new(
            MockLlmProvider::scripted(|_| Ok(LlmResponse::default()))
                .with_model_name("ghost")
                .failing_validation(LlmError::ModelNotFound("ghost".into())),
        );
        let router = RoutingProvider::new(a, HashMap::from([("g2".to_string(), Arc::clone(&missing)), ("g3".to_string(), missing)]));
        let err = router.validate().await.unwrap_err();
        assert!(matches!(&err, LlmError::ModelNotFound(m) if m == "g2: ghost"), "{err:?}");
    }

    #[test]
    fn default_trait_methods_fall_back_to_the_single_model() {
        let p = MockLlmProvider::scripted(|_| Ok(LlmResponse::default())).with_model_name("only");
        assert_eq!(p.model_for(&LlmRequest::new("s", "u", &LlmParams::default(), CallKind::Vote).speaker("x")), "only");
        assert_eq!(p.capabilities_for(Some("x")).max_parallel_calls, p.capabilities().max_parallel_calls);
    }
}
