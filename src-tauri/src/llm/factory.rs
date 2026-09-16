//! Builds the configured [`LlmProvider`] from application settings.

use std::sync::Arc;

use super::deepseek::DeepSeekProvider;
use super::ollama::OllamaProvider;
use super::{LlmError, LlmProvider};
use crate::constants;
use crate::models::llm::ProviderKind;
use crate::models::settings::AppSettings;

/// Instantiate and validate the provider selected in settings.
///
/// Validation covers what a discussion needs to start: the model exists
/// (Ollama) or the API key is accepted (DeepSeek).
pub async fn build_provider(settings: &AppSettings) -> Result<Arc<dyn LlmProvider>, LlmError> {
    let provider: Arc<dyn LlmProvider> = match settings.llm_provider {
        ProviderKind::Ollama => {
            if settings.ollama_model.is_empty() {
                return Err(LlmError::ModelNotFound("No Ollama model configured".to_string()));
            }
            Arc::new(OllamaProvider::connect(&settings.ollama_url, &settings.ollama_model, settings.num_ctx).await)
        }
        ProviderKind::DeepSeek => {
            if settings.deepseek_api_key.trim().is_empty() {
                return Err(LlmError::Auth);
            }
            Arc::new(DeepSeekProvider::new(
                &settings.deepseek_api_key,
                &settings.deepseek_model,
                deepseek_context_budget(settings.num_ctx),
            )?)
        }
    };
    provider.validate().await?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_budget_is_clamped() {
        assert_eq!(deepseek_context_budget(0), constants::DEEPSEEK_DEFAULT_CONTEXT_BUDGET);
        assert_eq!(deepseek_context_budget(10), constants::DEEPSEEK_MIN_CONTEXT_BUDGET);
        assert_eq!(deepseek_context_budget(50_000), 50_000);
        assert_eq!(deepseek_context_budget(10_000_000), constants::DEEPSEEK_MAX_CONTEXT_BUDGET);
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
    }
}
