//! Provider-level commands: DeepSeek model discovery, key validation and
//! monthly spend tracking.

use serde::Serialize;
use tauri::State;

use crate::constants;
use crate::db::{repository, rolling_period};
use crate::error::CommandError;
use crate::llm::deepseek::{DeepSeekBalance, DeepSeekProvider};
use crate::llm::pricing;
use crate::models::llm::{PeriodHistoryEntry, PeriodUsage, ProviderKind};
use crate::state::AppState;

/// Model ids available on the DeepSeek account.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeepSeekModels {
    pub models: Vec<String>,
    /// False when the documented fallback list was used (API unreachable).
    pub from_api: bool,
}

/// Peak-hour price list of one model (USD per million tokens).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelPriceInfo {
    pub model: String,
    pub input_cache_hit: f64,
    pub input_cache_miss: f64,
    pub output: f64,
}

/// Provider limits and defaults the frontend must not duplicate.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmConstants {
    pub deepseek_default_model: String,
    pub deepseek_known_models: Vec<String>,
    pub deepseek_default_context_budget: u32,
    pub deepseek_min_context_budget: u32,
    pub deepseek_max_context_budget: u32,
    pub deepseek_max_num_predict_ui: i32,
    pub deepseek_pricing_date: String,
    /// Known models' peak prices (cost previews in the setup wizard)
    pub deepseek_pricing: Vec<ModelPriceInfo>,
    pub deepseek_offpeak_factor: f64,
    /// UTC hour windows `[start, end)` of peak pricing, Monday–Friday
    pub deepseek_peak_windows_utc: Vec<[u32; 2]>,
    pub deepseek_top_p_min_thinking: f32,
    pub budget_warn_ratio: f64,
}

#[tauri::command]
pub fn get_llm_constants() -> LlmConstants {
    let deepseek_pricing = constants::DEEPSEEK_KNOWN_MODELS
        .iter()
        .filter_map(|m| {
            pricing::pricing_for(m).map(|p| ModelPriceInfo {
                model: m.to_string(),
                input_cache_hit: p.input_cache_hit,
                input_cache_miss: p.input_cache_miss,
                output: p.output,
            })
        })
        .collect();
    LlmConstants {
        deepseek_default_model: constants::DEEPSEEK_DEFAULT_MODEL.to_string(),
        deepseek_known_models: constants::DEEPSEEK_KNOWN_MODELS.iter().map(|s| s.to_string()).collect(),
        deepseek_default_context_budget: constants::DEEPSEEK_DEFAULT_CONTEXT_BUDGET,
        deepseek_min_context_budget: constants::DEEPSEEK_MIN_CONTEXT_BUDGET,
        deepseek_max_context_budget: constants::DEEPSEEK_MAX_CONTEXT_BUDGET,
        deepseek_max_num_predict_ui: constants::DEEPSEEK_MAX_NUM_PREDICT_UI,
        deepseek_pricing_date: constants::DEEPSEEK_PRICING_DATE.to_string(),
        deepseek_pricing,
        deepseek_offpeak_factor: constants::DEEPSEEK_OFFPEAK_FACTOR,
        deepseek_peak_windows_utc: constants::DEEPSEEK_PEAK_WINDOWS_UTC.iter().map(|(s, e)| [*s, *e]).collect(),
        deepseek_top_p_min_thinking: constants::DEEPSEEK_TOP_P_MIN_THINKING,
        budget_warn_ratio: constants::LLM_BUDGET_WARN_RATIO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_expose_pricing_for_every_known_model() {
        let c = get_llm_constants();
        assert_eq!(c.deepseek_pricing.len(), constants::DEEPSEEK_KNOWN_MODELS.len(), "every known model must be priced");
        assert!(c.deepseek_pricing.iter().all(|p| p.output > p.input_cache_miss && p.input_cache_miss > p.input_cache_hit));
        assert_eq!(c.deepseek_peak_windows_utc.len(), constants::DEEPSEEK_PEAK_WINDOWS_UTC.len());
        assert!(c.deepseek_known_models.contains(&c.deepseek_default_model));
    }
}

/// Current rolling period of cloud spend (Settings gauge + pre-flight checks).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmUsagePeriod {
    pub provider: ProviderKind,
    pub model: String,
    pub period_start: String,
    pub period_end: String,
    pub usage: PeriodUsage,
    /// Monthly cap in USD (0 = unlimited).
    pub budget_usd: f64,
    pub history: Vec<PeriodHistoryEntry>,
    pub pricing_date: String,
    /// Whether peak pricing applies right now.
    pub peak_now: bool,
}

fn map_llm_err(e: crate::llm::LlmError) -> CommandError {
    CommandError::Llm(e.to_string())
}

/// Resolve the key to use: explicit argument (Settings page, unsaved) or stored one.
async fn effective_key(state: &State<'_, AppState>, api_key: Option<String>) -> Result<String, CommandError> {
    let key = match api_key.map(|k| k.trim().to_string()) {
        Some(k) if !k.is_empty() => k,
        _ => state.get_settings().await?.deepseek_api_key,
    };
    if key.trim().is_empty() {
        return Err(CommandError::Llm("No DeepSeek API key configured".to_string()));
    }
    Ok(key)
}

#[tauri::command]
pub async fn list_deepseek_models(
    api_key: Option<String>,
    state: State<'_, AppState>,
) -> Result<DeepSeekModels, CommandError> {
    let key = effective_key(&state, api_key).await?;
    let provider = DeepSeekProvider::new(&key, constants::DEEPSEEK_DEFAULT_MODEL, constants::DEEPSEEK_DEFAULT_CONTEXT_BUDGET)
        .map_err(map_llm_err)?;
    match provider.list_models().await {
        Ok(models) => Ok(DeepSeekModels { models, from_api: true }),
        // Auth problems must surface; anything else degrades to the documented list.
        Err(crate::llm::LlmError::Auth) => Err(map_llm_err(crate::llm::LlmError::Auth)),
        Err(e) => {
            tracing::warn!(error = %e, "DeepSeek model list unavailable — using documented fallback");
            Ok(DeepSeekModels {
                models: constants::DEEPSEEK_KNOWN_MODELS.iter().map(|s| s.to_string()).collect(),
                from_api: false,
            })
        }
    }
}

/// Validate a key against `GET /user/balance` (cheapest authenticated call).
#[tauri::command]
pub async fn validate_deepseek_key(
    api_key: String,
    state: State<'_, AppState>,
) -> Result<DeepSeekBalance, CommandError> {
    let key = effective_key(&state, Some(api_key)).await?;
    let provider = DeepSeekProvider::new(&key, constants::DEEPSEEK_DEFAULT_MODEL, constants::DEEPSEEK_DEFAULT_CONTEXT_BUDGET)
        .map_err(map_llm_err)?;
    provider.balance().await.map_err(map_llm_err)
}

async fn build_period(state: &State<'_, AppState>) -> Result<LlmUsagePeriod, CommandError> {
    let db = state.db.clone();
    repository::check_and_reset_deepseek_period(&db)
        .await
        .map_err(|e| CommandError::Settings(e.to_string()))?;
    let settings = state.get_settings().await?;
    let usage: PeriodUsage = serde_json::from_str(&settings.deepseek_period_usage_json).unwrap_or_default();
    let history: Vec<PeriodHistoryEntry> = serde_json::from_str(&settings.deepseek_usage_history).unwrap_or_default();
    let period_end = rolling_period::parse_period_start(&settings.deepseek_period_start)
        .map(|d| rolling_period::period_end(d).format("%Y-%m-%d").to_string())
        .unwrap_or_default();
    Ok(LlmUsagePeriod {
        provider: settings.llm_provider,
        model: settings.deepseek_model,
        period_start: settings.deepseek_period_start,
        period_end,
        usage,
        budget_usd: settings.deepseek_monthly_budget_usd,
        history,
        pricing_date: constants::DEEPSEEK_PRICING_DATE.to_string(),
        peak_now: pricing::is_peak_hour(chrono::Utc::now()),
    })
}

#[tauri::command]
pub async fn get_llm_usage_period(state: State<'_, AppState>) -> Result<LlmUsagePeriod, CommandError> {
    build_period(&state).await
}

#[tauri::command]
pub async fn reset_llm_usage_period(state: State<'_, AppState>) -> Result<LlmUsagePeriod, CommandError> {
    let db = state.db.clone();
    repository::reset_deepseek_period_usage(&db)
        .await
        .map_err(|e| CommandError::Settings(e.to_string()))?;
    build_period(&state).await
}
