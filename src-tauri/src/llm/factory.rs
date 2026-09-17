//! Builds the configured [`LlmProvider`] from application settings — and, when
//! speakers override the model, a [`RoutingProvider`] with one inner provider
//! per distinct model (v1.20).

use std::collections::HashMap;
use std::sync::Arc;

use super::deepseek::DeepSeekProvider;
use super::ollama::OllamaProvider;
use super::openai_compat::OpenAiCompatProvider;
use super::routing::RoutingProvider;
use super::{LlmError, LlmProvider};
use crate::constants;
use crate::models::llm::ProviderKind;
use crate::models::settings::AppSettings;

/// A speaker's model override, as read from the discussion config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeakerModel {
    pub speaker_id: String,
    pub model: String,
}

/// Instantiate and validate the provider selected in settings.
///
/// Validation covers what a discussion needs to start: the model exists
/// (Ollama, OpenAI-compatible catalogue) or the API key is accepted (DeepSeek).
pub async fn build_provider(settings: &AppSettings) -> Result<Arc<dyn LlmProvider>, LlmError> {
    let provider = build_for_model(settings, global_model(settings)).await?;
    provider.validate().await?;
    Ok(provider)
}

/// The provider of a discussion: the global model, plus one inner provider per
/// distinct speaker model when some speakers override it. Every model is
/// validated before the engine starts; a speaker's failure names the speaker.
pub async fn build_provider_for(settings: &AppSettings, overrides: &[SpeakerModel]) -> Result<Arc<dyn LlmProvider>, LlmError> {
    let default_model = global_model(settings);
    let default = build_for_model(settings, default_model).await?;
    default.validate().await?;
    let mut by_model: HashMap<String, Arc<dyn LlmProvider>> = HashMap::new();
    let mut by_speaker: HashMap<String, Arc<dyn LlmProvider>> = HashMap::new();
    for o in overrides {
        let model = o.model.trim();
        if model.is_empty() || model == default_model {
            continue;
        }
        let provider = match by_model.get(model) {
            Some(p) => Arc::clone(p),
            None => {
                let p = build_for_model(settings, model).await?;
                p.validate().await.map_err(|e| match e {
                    LlmError::ModelNotFound(m) => LlmError::ModelNotFound(format!("{}: {m}", o.speaker_id)),
                    other => other,
                })?;
                by_model.insert(model.to_string(), Arc::clone(&p));
                p
            }
        };
        by_speaker.insert(o.speaker_id.clone(), provider);
    }
    if by_speaker.is_empty() {
        return Ok(default);
    }
    tracing::info!(models = ?by_model.keys().collect::<Vec<_>>(), speakers = by_speaker.len(), "Multi-model routing enabled");
    Ok(Arc::new(RoutingProvider::new(default, by_speaker)))
}

/// The model the settings designate for the selected provider.
fn global_model(settings: &AppSettings) -> &str {
    match settings.llm_provider {
        ProviderKind::Ollama => &settings.ollama_model,
        ProviderKind::DeepSeek => &settings.deepseek_model,
        ProviderKind::OpenAiCompat => &settings.openai_compat_model,
    }
}

/// One (unvalidated) provider of the selected kind for `model`.
async fn build_for_model(settings: &AppSettings, model: &str) -> Result<Arc<dyn LlmProvider>, LlmError> {
    let provider: Arc<dyn LlmProvider> = match settings.llm_provider {
        ProviderKind::Ollama => {
            if model.trim().is_empty() {
                return Err(LlmError::ModelNotFound("No Ollama model configured".to_string()));
            }
            Arc::new(OllamaProvider::connect(&settings.ollama_url, model, settings.num_ctx).await)
        }
        ProviderKind::DeepSeek => {
            if settings.deepseek_api_key.trim().is_empty() {
                return Err(LlmError::Auth);
            }
            Arc::new(DeepSeekProvider::new(&settings.deepseek_api_key, model, deepseek_context_budget(settings.num_ctx))?)
        }
        ProviderKind::OpenAiCompat => Arc::new(OpenAiCompatProvider::new(
            &settings.openai_compat_base_url,
            &settings.openai_compat_api_key,
            model,
            openai_compat_context_tokens(settings.num_ctx),
        )?),
    };
    Ok(provider)
}

/// Clamp the user's context budget to the DeepSeek bounds (a 0/legacy value
/// falls back to the default budget).
pub fn deepseek_context_budget(num_ctx: u32) -> u32 {
    if num_ctx == 0 {
        return constants::DEEPSEEK_DEFAULT_CONTEXT_BUDGET;
    }
    num_ctx.clamp(constants::DEEPSEEK_MIN_CONTEXT_BUDGET, constants::DEEPSEEK_MAX_CONTEXT_BUDGET)
}

/// Context window of a generic server: the user's setting, or a sane default.
pub fn openai_compat_context_tokens(num_ctx: u32) -> u32 {
    if num_ctx == 0 { constants::OPENAI_COMPAT_DEFAULT_CONTEXT_TOKENS } else { num_ctx }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_budget_is_clamped() {
        assert_eq!(deepseek_context_budget(0), constants::DEEPSEEK_DEFAULT_CONTEXT_BUDGET);
        assert_eq!(deepseek_context_budget(10), constants::DEEPSEEK_MIN_CONTEXT_BUDGET);
        assert_eq!(deepseek_context_budget(50_000), 50_000);
        assert_eq!(deepseek_context_budget(10_000_000), constants::DEEPSEEK_MAX_CONTEXT_BUDGET);
        assert_eq!(openai_compat_context_tokens(0), constants::OPENAI_COMPAT_DEFAULT_CONTEXT_TOKENS);
        assert_eq!(openai_compat_context_tokens(8192), 8192);
    }

    #[tokio::test]
    async fn build_provider_rejects_missing_configuration() {
        let mut s = AppSettings { llm_provider: ProviderKind::Ollama, ollama_model: String::new(), ..Default::default() };
        let err = build_provider(&s).await.err().expect("must fail");
        assert!(matches!(err, LlmError::ModelNotFound(_)));

        s.llm_provider = ProviderKind::DeepSeek;
        s.deepseek_api_key = "  ".to_string();
        let err = build_provider(&s).await.err().expect("must fail");
        assert!(matches!(err, LlmError::Auth));

        s.llm_provider = ProviderKind::OpenAiCompat;
        s.openai_compat_model = String::new();
        let err = build_provider(&s).await.err().expect("must fail");
        assert!(matches!(err, LlmError::ModelNotFound(_)));
    }

    /// Overrides equal to the global model or blank never create a router (the
    /// speaker-naming of a failed override is covered by `routing::tests`, S24).
    #[tokio::test]
    async fn overrides_matching_the_global_model_keep_the_plain_provider() {
        // A reachable server without catalogue (404): the validation tolerates it
        let (base, _) = super::super::openai_compat::test_support::serve_once("404 Not Found", "{}").await;
        let s = AppSettings { llm_provider: ProviderKind::OpenAiCompat, openai_compat_model: "m".into(), openai_compat_base_url: base, ..Default::default() };
        let same = vec![SpeakerModel { speaker_id: "g1".into(), model: "m".into() }, SpeakerModel { speaker_id: "g2".into(), model: " ".into() }];
        let p = build_provider_for(&s, &same).await.expect("same model, no router");
        assert_eq!(p.model_name(), "m");
        assert_eq!(p.model_for(&super::super::LlmRequest::new("s", "u", &Default::default(), crate::models::llm::CallKind::Vote).speaker("g1")), "m");
    }
}
