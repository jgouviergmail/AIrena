use std::sync::Arc;
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::State;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::db::repository;
use crate::engine::orchestrator::DiscussionEngine;
use crate::engine::token_budget;
use crate::error::CommandError;
use crate::llm::factory;
use crate::models::discussion::DiscussionConfig;
use crate::models::message::ReactionType;
use crate::models::engine_command::EngineCommand;
use crate::models::events::ArenaEvent;
use crate::state::AppState;

#[tauri::command]
pub async fn start_discussion(
    config: DiscussionConfig,
    on_event: Channel<ArenaEvent>,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    // Validate config
    if config.user_name.trim().is_empty() {
        return Err(CommandError::Settings("Username must not be empty".to_string()));
    }
    if config.topic.trim().is_empty() {
        return Err(CommandError::Settings("Topic must not be empty".to_string()));
    }
    if config.gladiateurs.is_empty() {
        return Err(CommandError::Settings("At least one gladiator is required".to_string()));
    }

    // Create command channel and CancellationToken UPFRONT
    let (cmd_tx, cmd_rx) = mpsc::channel::<EngineCommand>(32);
    let cancel_token = CancellationToken::new();
    let engine_cancel = cancel_token.clone();

    // ATOMIC check-and-reserve: prevents TOCTOU race where two near-simultaneous
    // calls could both see None before either sets Some.
    {
        let mut tx_guard = AppState::lock_or_recover(&state.engine_cmd_tx);
        if tx_guard.is_some() {
            return Err(CommandError::AlreadyRunning);
        }
        // Reserve the slot immediately
        *tx_guard = Some(cmd_tx);
    }
    {
        let mut cancel_guard = AppState::lock_or_recover(&state.cancel_token);
        *cancel_guard = Some(cancel_token);
    }

    let cleanup_tx = Arc::clone(&state.engine_cmd_tx);
    let cleanup_cancel = Arc::clone(&state.cancel_token);

    // Read settings for ollama_url and ollama_model
    let settings = match state.get_settings().await {
        Ok(s) => s,
        Err(e) => {
            AppState::clear_engine_slots(&cleanup_tx, &cleanup_cancel);
            return Err(e);
        }
    };

    // ── License gate (validate only, no counter mutation) ──────────
    if settings.license_key.is_empty() {
        AppState::clear_engine_slots(&cleanup_tx, &cleanup_cancel);
        return Err(CommandError::License("No license key configured".to_string()));
    }
    let db_for_license = state.db.clone();
    {
        let (stored_hash, disc_count, last_check) =
            repository::get_license_tracking(&db_for_license).await.map_err(|e| {
                AppState::clear_engine_slots(&cleanup_tx, &cleanup_cancel);
                CommandError::License(e.to_string())
            })?;
        let status = crate::license::check_license_status(
            &settings.license_key, &stored_hash, disc_count, last_check,
        );
        if !status.valid {
            AppState::clear_engine_slots(&cleanup_tx, &cleanup_cancel);
            return Err(CommandError::License(
                status.error.unwrap_or_else(|| "Invalid license".to_string()),
            ));
        }
    }

    // ── Monthly cloud budget pre-flight (DeepSeek) ─────────────────
    let period_spent_usd = match super::llm::cloud_budget_preflight(&state.db, &settings).await {
        Ok(spent) => spent,
        Err(e) => {
            AppState::clear_engine_slots(&cleanup_tx, &cleanup_cancel);
            return Err(e);
        }
    };
    let monthly_budget_usd = settings.deepseek_monthly_budget_usd;

    // Build and validate the configured LLM provider (model exists / key accepted),
    // with one inner provider per speaker model override (v1.20)
    let overrides: Vec<factory::SpeakerModel> = config
        .gladiateurs
        .iter()
        .filter_map(|g| g.model.as_deref().map(|m| factory::SpeakerModel { speaker_id: g.id.clone(), model: m.to_string() }))
        .chain(config.arbitre.model.as_deref().map(|m| factory::SpeakerModel { speaker_id: config.arbitre.id.clone(), model: m.to_string() }))
        .collect();
    let provider = match factory::build_provider_for(&settings, &overrides).await {
        Ok(p) => p,
        Err(e) => {
            AppState::clear_engine_slots(&cleanup_tx, &cleanup_cancel);
            return Err(CommandError::Llm(e.to_string()));
        }
    };

    // ── Increment license counter (after all pre-flight checks passed) ──
    {
        let key_hash = crate::license::hash_license_key(&settings.license_key);
        if let Err(e) = repository::increment_license_discussions(&db_for_license, &key_hash).await {
            AppState::clear_engine_slots(&cleanup_tx, &cleanup_cancel);
            return Err(CommandError::License(e.to_string()));
        }
    }

    // Check/reset Tavily period if API key is configured
    if !settings.tavily_api_key.is_empty() {
        if let Err(e) = repository::check_and_reset_tavily_period(&state.db).await {
            tracing::warn!(error = %e, "Failed to check/reset Tavily billing period — continuing with current counts");
        }
    }

    // Spawn the engine on the Tauri async runtime (non-blocking)
    let discussion_id = uuid::Uuid::new_v4().to_string();
    let id_clone = discussion_id.clone();
    let emotion_driven = settings.emotion_driven;
    let reasoning_level = settings.reasoning_level;
    let show_model_reasoning = settings.show_model_reasoning;
    let reasoning_pace = settings.reasoning_pace;
    let tavily_key = if settings.tavily_api_key.is_empty() {
        None
    } else {
        Some(settings.tavily_api_key.clone())
    };
    let db_clone = state.db.clone();

    // Take RAG store from AppState (ownership transfer to engine)
    let rag_store = AppState::lock_or_recover(&state.rag_store).take();

    let argument_map_enabled = config.argument_map_enabled;

    let priorities = token_budget::parse_priorities_or_default(&settings.token_budget_priorities);

    let tuning = crate::engine::tuning::Tuning::from_settings(&settings.advanced_tuning_json);
    let persona_memory_enabled = settings.persona_memory_enabled;

    tauri::async_runtime::spawn(async move {
        let mut engine = DiscussionEngine::new(
            config, id_clone, provider,
            tavily_key.as_deref(), db_clone, rag_store, priorities,
        );
        engine.set_cancel_token(engine_cancel);
        engine.set_emotion_driven(emotion_driven);
        engine.set_tuning(tuning);
        engine.set_persona_memory(persona_memory_enabled);
        engine.set_reasoning_options(reasoning_level, show_model_reasoning, reasoning_pace);
        engine.set_budget_guard(period_spent_usd, monthly_budget_usd);
        engine.set_argument_map_enabled(argument_map_enabled);
        engine.run(cmd_rx, on_event).await;

        // Cleanup: remove sender and token so a new discussion can start
        AppState::clear_engine_slots(&cleanup_tx, &cleanup_cancel);
    });

    Ok(discussion_id)
}

#[tauri::command]
pub async fn pause_discussion(state: State<'_, AppState>) -> Result<(), CommandError> {
    state.send_engine_command(EngineCommand::Pause).await
}

#[tauri::command]
pub async fn resume_discussion(state: State<'_, AppState>) -> Result<(), CommandError> {
    state.send_engine_command(EngineCommand::Resume).await
}

#[tauri::command]
pub async fn stop_discussion(state: State<'_, AppState>) -> Result<(), CommandError> {
    state.send_engine_command(EngineCommand::Stop).await
}

#[tauri::command]
pub async fn force_stop_discussion(state: State<'_, AppState>) -> Result<(), CommandError> {
    // 1. Send ForceStop via mpsc (best-effort, ignore if channel closed)
    let tx = AppState::lock_or_recover(&state.engine_cmd_tx).clone();
    if let Some(tx) = tx {
        let _ = tx.send(EngineCommand::ForceStop).await;
    }
    // 2. Cancel via CancellationToken (cuts any in-progress streaming)
    let cancel = AppState::lock_or_recover(&state.cancel_token).take();
    if let Some(token) = cancel {
        token.cancel();
    }
    Ok(())
}

#[tauri::command]
pub async fn user_wants_to_intervene(state: State<'_, AppState>) -> Result<(), CommandError> {
    state
        .send_engine_command(EngineCommand::UserWantsToIntervene)
        .await
}

#[tauri::command]
pub async fn submit_user_message(
    content: String,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    state
        .send_engine_command(EngineCommand::SubmitUserMessage { content })
        .await
}

#[tauri::command]
pub async fn skip_user_turn(state: State<'_, AppState>) -> Result<(), CommandError> {
    state
        .send_engine_command(EngineCommand::SkipUserTurn)
        .await
}

/// Engine limits the frontend mirrors in its UI (owned by the backend, never duplicated).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineConstants {
    pub audience_reactions_per_message_max: u32,
    /// Chars the secret agenda block takes in the system prompt (budget preview, v1.19)
    pub agenda_block_max_chars: usize,
    /// Gladiateurs a casting may suggest, at most (v1.19)
    pub casting_max_gladiateurs: u32,
}

#[tauri::command]
pub fn get_engine_constants() -> EngineConstants {
    EngineConstants {
        audience_reactions_per_message_max: crate::constants::AUDIENCE_REACTIONS_PER_MESSAGE_MAX,
        agenda_block_max_chars: crate::constants::AGENDA_MAX_CHARS + crate::constants::AGENDA_BLOCK_OVERHEAD_CHARS,
        casting_max_gladiateurs: crate::constants::CASTING_MAX_GLADIATEURS,
    }
}

/// The audience (the user) reacts to a message of the running discussion.
#[tauri::command]
pub async fn react_to_message(
    message_id: String,
    reaction_type: ReactionType,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    if message_id.trim().is_empty() {
        return Err(CommandError::Settings("Message id must not be empty".to_string()));
    }
    state
        .send_engine_command(EngineCommand::AudienceReaction { message_id, reaction_type })
        .await
}

/// The audience votes on the motion of an Oxford debate (`for` / `against`).
#[tauri::command]
pub async fn audience_vote(choice: String, state: State<'_, AppState>) -> Result<(), CommandError> {
    let choice = choice.trim().to_lowercase();
    if choice != crate::models::outcome::VOTE_FOR && choice != crate::models::outcome::VOTE_AGAINST {
        return Err(CommandError::Settings(format!("Unknown vote choice: {choice}")));
    }
    state.send_engine_command(EngineCommand::AudienceVote { choice }).await
}

/// Step mode (v1.20.1): the engine waits for the audience's cue before each speaker
/// (the voice in "follow" mode reads at its own pace).
#[tauri::command]
pub async fn set_step_mode(enabled: bool, state: State<'_, AppState>) -> Result<(), CommandError> {
    state.send_engine_command(EngineCommand::SetStepMode { enabled }).await
}

/// The audience's cue: the next speaker may talk.
#[tauri::command]
pub async fn next_speaker(state: State<'_, AppState>) -> Result<(), CommandError> {
    state.send_engine_command(EngineCommand::NextSpeaker).await
}

#[tauri::command]
pub async fn adjust_emotion(
    speaker_id: String,
    axis: String,
    value: u8,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    state
        .send_engine_command(EngineCommand::AdjustEmotion {
            speaker_id,
            axis,
            value,
        })
        .await
}
