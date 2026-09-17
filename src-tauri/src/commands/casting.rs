//! Assisted casting (v1.19): the model picks, from the profile catalogue, the
//! gladiateurs and the moderator that will make a topic the most lively.

use tauri::State;
use tokio_util::sync::CancellationToken;

use crate::constants;
use crate::db::repository;
use crate::engine::json_parser;
use crate::engine::prompt_builder::{self, CastingCandidate};
use crate::engine::token_budget;
use crate::error::CommandError;
use crate::llm::factory;
use crate::llm::metered::MeteredProvider;
use crate::llm::{LlmProvider, LlmRequest};
use crate::models::agenda::CastingSuggestion;
use crate::models::discussion::DiscussionMode;
use crate::models::llm::CallKind;
use crate::models::profile::PredefinedProfile;
use crate::models::settings::{AppSettings, LlmParams};
use crate::state::AppState;

/// Catalogue chars the casting prompt may hold for this context window: the
/// output and the instructions are taken out first, then the hard cap applies.
pub fn catalogue_bound(num_ctx: u32, chars_per_token: f64) -> usize {
    let context_chars = (f64::from(num_ctx) * chars_per_token).floor() as usize;
    let reserved = (f64::from(constants::CASTING_NUM_PREDICT.max(0)) * chars_per_token).ceil() as usize + constants::CASTING_INSTRUCTIONS_CHARS;
    context_chars.saturating_sub(reserved).min(constants::CASTING_CATALOGUE_MAX_CHARS)
}

fn candidates(profiles: &[PredefinedProfile]) -> Vec<CastingCandidate<'_>> {
    profiles.iter().map(|p| CastingCandidate { id: &p.id, name: &p.name, personality: &p.personality }).collect()
}

/// Suggest `count` gladiateurs and a moderator for a topic. An empty catalogue
/// yields an empty suggestion; unknown ids in the answer are dropped.
#[tauri::command]
pub async fn suggest_casting(
    topic: String,
    mode: DiscussionMode,
    lang: String,
    count: u32,
    state: State<'_, AppState>,
) -> Result<CastingSuggestion, CommandError> {
    if topic.trim().is_empty() {
        return Err(CommandError::Settings("Topic must not be empty".to_string()));
    }
    let count = count.clamp(1, constants::CASTING_MAX_GLADIATEURS);
    let db = state.db.clone();
    let settings = state.get_settings().await?;

    let gladiateurs = repository::list_profiles(&db).await.map_err(|e| CommandError::Settings(e.to_string()))?;
    let arbitres = repository::list_arbitre_profiles(&db).await.map_err(|e| CommandError::Settings(e.to_string()))?;
    if gladiateurs.is_empty() {
        tracing::warn!("Casting requested on an empty profile catalogue");
        return Ok(CastingSuggestion::default());
    }

    super::llm::cloud_budget_preflight(&db, &settings).await?;
    let provider = factory::build_provider(&settings).await.map_err(|e| CommandError::Llm(e.to_string()))?;
    let llm = MeteredProvider::new(provider);
    let chars_per_token = token_budget::chars_per_token_for_language(&lang, llm.kind());
    let bound = catalogue_bound(llm.capabilities().context_tokens, chars_per_token);
    let (system, user) = prompt_builder::build_casting_prompt(
        &topic, &mode, &lang, count, &candidates(&gladiateurs), &candidates(&arbitres), bound,
    );
    let request = LlmRequest::new(&system, &user, &casting_params(&settings), CallKind::Casting).json();
    let result = llm.chat(&request, CancellationToken::new()).await;
    record_cloud_usage(&db, &llm).await;
    let raw = result.map_err(|e| CommandError::Llm(e.to_string()))?.content;

    let glad_ids: Vec<String> = gladiateurs.iter().map(|p| p.id.clone()).collect();
    let arb_ids: Vec<String> = arbitres.iter().map(|p| p.id.clone()).collect();
    let suggestion = json_parser::parse_casting(&raw, &glad_ids, &arb_ids);
    tracing::info!(picked = suggestion.gladiateurs.len(), arbitre = ?suggestion.arbitre, count, "Casting suggested");
    Ok(suggestion)
}

/// Sampling of the casting call: the global context window, a short JSON output.
fn casting_params(settings: &AppSettings) -> LlmParams {
    LlmParams { num_ctx: settings.num_ctx, num_predict: constants::CASTING_NUM_PREDICT, ..LlmParams::default() }
}

/// Cloud tokens of the call join the monthly period (never lost, even on error).
async fn record_cloud_usage(db: &tokio_rusqlite::Connection, llm: &MeteredProvider) {
    if !llm.capabilities().billable {
        return;
    }
    let ledger = llm.snapshot();
    if ledger.total.is_empty() {
        return;
    }
    if let Err(e) = repository::record_deepseek_usage(db, &ledger.total, ledger.estimated_cost_usd.unwrap_or(0.0)).await {
        tracing::warn!(error = %e, "Failed to record the casting call into the monthly period");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_bound_leaves_room_for_the_answer_and_never_exceeds_the_cap() {
        // 4k context, 4 chars/token: 16 384 chars minus the output (2 048) and the instructions
        let bound = catalogue_bound(4_096, 4.0);
        assert_eq!(bound, 16_384 - 2_048 - constants::CASTING_INSTRUCTIONS_CHARS);
        // Huge context → hard cap
        assert_eq!(catalogue_bound(131_072, 4.0), constants::CASTING_CATALOGUE_MAX_CHARS);
        // Tiny context → nothing left, no underflow
        assert_eq!(catalogue_bound(256, 4.0), 0);
    }
}
