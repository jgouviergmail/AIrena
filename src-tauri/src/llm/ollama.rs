//! Ollama adapter — wraps [`OllamaClient`] behind the [`LlmProvider`] trait.
//!
//! Wire mapping is **iso-functional** with the historical engine behaviour:
//! - `think` is never sent (`None`): Ollama's native protocol lets thinking models
//!   reason by default and separates reasoning into the `thinking` field.
//!   `think: Some(false)` must never be used — it makes models dump raw reasoning
//!   into the content field.
//! - For thinking models, discussion-content calls (introduction, thought,
//!   intervention) get `num_predict × THINK_NUM_PREDICT_MULTIPLIER` because
//!   `num_predict` caps thinking + content together.
//! - Reasoning text is discarded from display (raw meta-reasoning, not in-character).

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use super::{LlmCapabilities, LlmError, LlmProvider, LlmRequest, LlmResponse, TokenCallback};
use crate::constants;
use crate::models::llm::{LlmUsage, ProviderKind};
use crate::ollama::client::{strip_think_tags, OllamaClient};
use crate::ollama::model_info;
use crate::ollama::types::{ChatMessage, ChatOptions, ChatRequest};

pub struct OllamaProvider {
    client: OllamaClient,
    caps: LlmCapabilities,
}

impl OllamaProvider {
    /// Build the adapter and detect think support from the model template.
    /// Detection failure is not fatal (logged, treated as "no reasoning").
    pub async fn connect(base_url: &str, model: &str, num_ctx: u32) -> Self {
        let client = OllamaClient::new(base_url, model);
        let supports_reasoning = match client.show_model(model).await {
            Ok(show) => model_info::detect_think_support(&show.template),
            Err(e) => {
                tracing::warn!(error = %e, "Could not detect Ollama think support — assuming none");
                false
            }
        };
        tracing::info!(model, supports_reasoning, "Ollama provider ready");
        Self::with_capabilities(client, supports_reasoning, num_ctx)
    }

    pub fn with_capabilities(client: OllamaClient, supports_reasoning: bool, num_ctx: u32) -> Self {
        Self {
            client,
            caps: LlmCapabilities {
                supports_reasoning,
                reasoning_levels: false,
                reasoning_displayable: false,
                supports_json_mode: true,
                context_tokens: num_ctx,
                chars_per_token_latin: constants::CHARS_PER_TOKEN_LATIN,
                chars_per_token_cjk: constants::CHARS_PER_TOKEN_CJK,
                reports_usage: true,
                billable: false,
                max_parallel_calls: constants::OLLAMA_MAX_PARALLEL_CALLS,
            },
        }
    }

    /// Translate a generic request into Ollama's `/api/chat` body.
    pub fn to_wire(&self, request: &LlmRequest) -> ChatRequest {
        let mut messages = Vec::with_capacity(2);
        if !request.system.is_empty() {
            messages.push(ChatMessage { role: "system".to_string(), content: request.system.clone() });
        }
        messages.push(ChatMessage { role: "user".to_string(), content: request.user.clone() });

        let params = &request.params;
        let num_predict = if self.caps.supports_reasoning && request.call_kind.is_discussion_content() {
            params.num_predict.saturating_mul(constants::THINK_NUM_PREDICT_MULTIPLIER)
        } else {
            params.num_predict
        };

        ChatRequest {
            model: self.client.model_name().to_string(),
            messages,
            format: request.json_mode.then(|| "json".to_string()),
            stream: true,
            options: Some(ChatOptions {
                temperature: Some(params.temperature),
                top_p: Some(params.top_p),
                top_k: Some(params.top_k),
                num_predict: Some(num_predict),
                num_ctx: Some(params.num_ctx),
                repeat_penalty: Some(params.repeat_penalty),
            }),
            think: None,
        }
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Ollama
    }

    fn model_name(&self) -> &str {
        self.client.model_name()
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
        let result = self
            .client
            .chat_streaming_with_think(&wire, on_content, on_reasoning, cancel)
            .await?;

        let usage = match (result.prompt_eval_count, result.eval_count) {
            (None, None) => None,
            (p, c) => Some(LlmUsage {
                prompt_tokens: p.unwrap_or(0),
                cached_tokens: 0,
                completion_tokens: c.unwrap_or(0),
                reasoning_tokens: 0,
            }),
        };

        Ok(LlmResponse {
            content: strip_think_tags(&result.content),
            reasoning: result.thinking.filter(|t| !t.is_empty()),
            usage,
            truncated: result.truncated,
        })
    }

    async fn validate(&self) -> Result<(), LlmError> {
        self.client.validate_model().await.map_err(LlmError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::llm::CallKind;
    use crate::models::settings::LlmParams;

    fn provider(supports_reasoning: bool) -> OllamaProvider {
        OllamaProvider::with_capabilities(
            OllamaClient::new("http://localhost:11434", "test-model"),
            supports_reasoning,
            8192,
        )
    }

    /// Golden test: the wire request must match the historical `build_request` output.
    #[test]
    fn wire_matches_legacy_build_request_for_utilities() {
        let params = LlmParams::default();
        let req = LlmRequest::new("SYS", "USER", &params, CallKind::Reaction).json();
        let wire = provider(true).to_wire(&req);
        let json = serde_json::to_value(&wire).unwrap();

        assert_eq!(json["model"], "test-model");
        assert_eq!(json["messages"][0]["role"], "system");
        assert_eq!(json["messages"][0]["content"], "SYS");
        assert_eq!(json["messages"][1]["role"], "user");
        assert_eq!(json["messages"][1]["content"], "USER");
        assert_eq!(json["format"], "json");
        assert_eq!(json["stream"], true);
        // JSON mode pins the structured-output temperature, not the persona's
        assert_eq!(json["options"]["temperature"], constants::TEMP_JSON_OUTPUT);
        assert_eq!(json["options"]["top_p"], params.top_p);
        assert_eq!(json["options"]["top_k"], params.top_k);
        assert_eq!(json["options"]["num_predict"], params.num_predict); // no multiplier on utilities
        assert_eq!(json["options"]["num_ctx"], params.num_ctx);
        assert_eq!(json["options"]["repeat_penalty"], params.repeat_penalty);
        assert!(json.get("think").is_none(), "think must never be sent");
    }

    #[test]
    fn wire_applies_think_multiplier_only_for_discussion_content_on_thinking_models() {
        let params = LlmParams::default();
        let expected = params.num_predict * constants::THINK_NUM_PREDICT_MULTIPLIER;

        for kind in [CallKind::Introduction, CallKind::Thought, CallKind::Intervention] {
            let wire = provider(true).to_wire(&LlmRequest::new("s", "u", &params, kind));
            assert_eq!(wire.options.unwrap().num_predict, Some(expected), "{kind:?}");
        }
        // Synthesis and utilities: no multiplier
        for kind in [CallKind::Synthesis, CallKind::Memory, CallKind::ArgumentMap] {
            let wire = provider(true).to_wire(&LlmRequest::new("s", "u", &params, kind));
            assert_eq!(wire.options.unwrap().num_predict, Some(params.num_predict), "{kind:?}");
        }
        // Non-thinking model: never
        let wire = provider(false).to_wire(&LlmRequest::new("s", "u", &params, CallKind::Intervention));
        assert_eq!(wire.options.unwrap().num_predict, Some(params.num_predict));
    }

    #[test]
    fn wire_omits_empty_system_prompt_and_json_format() {
        let params = LlmParams::default();
        let wire = provider(false).to_wire(&LlmRequest::new("", "u", &params, CallKind::Vote));
        assert_eq!(wire.messages.len(), 1);
        assert_eq!(wire.messages[0].role, "user");
        assert!(wire.format.is_none());
    }

    #[test]
    fn capabilities_reflect_detection() {
        let p = provider(true);
        assert!(p.capabilities().supports_reasoning);
        assert!(!p.capabilities().reasoning_levels);
        assert!(!p.capabilities().reasoning_displayable);
        assert!(!p.capabilities().billable);
        assert_eq!(p.capabilities().context_tokens, 8192);
        assert_eq!(p.kind(), ProviderKind::Ollama);
        assert_eq!(p.model_name(), "test-model");
    }
}
