use std::sync::Arc;
use tauri::ipc::Channel;
use tauri::State;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::db::repository;
use crate::engine::orchestrator::DiscussionEngine;
use crate::engine::token_budget;
use crate::error::CommandError;
use crate::models::discussion::DiscussionConfig;
use crate::models::engine_command::EngineCommand;
use crate::models::events::ArenaEvent;
use crate::ollama::client::OllamaClient;
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

    // Validate that the Ollama model exists
    let client = OllamaClient::new(&settings.ollama_url, &settings.ollama_model);
    if let Err(e) = client.validate_model().await {
        AppState::clear_engine_slots(&cleanup_tx, &cleanup_cancel);
        return Err(CommandError::Ollama(e.to_string()));
    }

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
    let ollama_url = settings.ollama_url.clone();
    let ollama_model = settings.ollama_model.clone();
    let emotion_driven = settings.emotion_driven;
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

    tauri::async_runtime::spawn(async move {
        let mut engine = DiscussionEngine::new(
            config, id_clone, &ollama_url, &ollama_model,
            tavily_key.as_deref(), db_clone, rag_store, priorities,
        );
        engine.set_cancel_token(engine_cancel);
        engine.set_emotion_driven(emotion_driven);
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
