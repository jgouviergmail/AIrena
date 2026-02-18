use std::time::Duration;

use tauri::State;

use crate::constants;
use crate::engine::token_budget::{self, BudgetParams, SectionPriority, TokenBudget, TokenBudgetPreview};
use crate::error::CommandError;
use crate::ollama::client::OllamaClient;
use crate::ollama::model_info::{self, ModelBudgetInfo};
use crate::ollama::types::ModelInfo;
use crate::state::AppState;

#[tauri::command]
pub async fn check_ollama_connection(state: State<'_, AppState>) -> Result<bool, CommandError> {
    let settings = state.get_settings().await?;
    let client = OllamaClient::new(&settings.ollama_url, "");
    Ok(client.check_connection().await)
}

#[tauri::command]
pub async fn list_ollama_models(state: State<'_, AppState>) -> Result<Vec<ModelInfo>, CommandError> {
    let settings = state.get_settings().await?;
    let client = OllamaClient::new(&settings.ollama_url, "");
    client
        .list_models()
        .await
        .map_err(|e| CommandError::Ollama(e.to_string()))
}

#[tauri::command]
pub async fn preload_ollama_model(
    model: String,
    num_ctx: Option<u32>,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    let settings = state.get_settings().await?;
    let client = OllamaClient::new(&settings.ollama_url, &model);
    client
        .preload_model(num_ctx)
        .await
        .map_err(|e| CommandError::Ollama(e.to_string()))
}

#[tauri::command]
pub async fn get_model_budget_info(
    model: String,
    state: State<'_, AppState>,
) -> Result<ModelBudgetInfo, CommandError> {
    let settings = state.get_settings().await?;
    let client = OllamaClient::new(&settings.ollama_url, &model);

    // 1. Model architecture
    let show = client
        .show_model(&model)
        .await
        .map_err(|e| CommandError::Ollama(e.to_string()))?;

    // 2. Model file sizes (file size ≈ VRAM for weights)
    let all_models = client
        .list_models()
        .await
        .map_err(|e| CommandError::Ollama(e.to_string()))?;
    let llm_size = model_info::find_model_file_size(&all_models, &model);
    let emb_size = if settings.embedding_model.is_empty() || settings.embedding_model == model {
        0 // Same model or no embedding — already counted
    } else {
        model_info::find_model_file_size(&all_models, &settings.embedding_model)
    };

    // 3. Currently loaded models' VRAM (for clean free reconstruction)
    let loaded_vram = client
        .list_running_models()
        .await
        .map(|ps| model_info::total_loaded_vram(&ps))
        .unwrap_or(0);

    Ok(model_info::build_model_budget_info(&show, llm_size + emb_size, loaded_vram).await)
}

/// Composite initialization command for app startup.
///
/// Sequence: check → get sizes → show → unload all → wait → detect VRAM (clean) →
/// recommend num_ctx → preload LLM → preload embedding → return ModelBudgetInfo.
#[tauri::command]
pub async fn initialize_ollama(
    state: State<'_, AppState>,
) -> Result<ModelBudgetInfo, CommandError> {
    let settings = state.get_settings().await?;
    let llm_model = settings.ollama_model.clone();
    let embedding_model = settings.embedding_model.clone();
    let ollama_url = settings.ollama_url.clone();

    if llm_model.is_empty() {
        return Err(CommandError::Ollama("No LLM model configured".to_string()));
    }

    let client = OllamaClient::new(&ollama_url, &llm_model);

    // 1. Check Ollama is reachable
    if !client.check_connection().await {
        return Err(CommandError::Ollama("Ollama is not reachable".to_string()));
    }

    // 2. Get model file sizes (file size ≈ VRAM for weights)
    let all_models = client
        .list_models()
        .await
        .map_err(|e| CommandError::Ollama(e.to_string()))?;
    let llm_size = model_info::find_model_file_size(&all_models, &llm_model);
    let emb_size = if embedding_model.is_empty() || embedding_model == llm_model {
        0
    } else {
        model_info::find_model_file_size(&all_models, &embedding_model)
    };

    // 3. Get model architecture (metadata only, doesn't load the model)
    let show = client
        .show_model(&llm_model)
        .await
        .map_err(|e| CommandError::Ollama(e.to_string()))?;

    // 4. Unload all models from VRAM to get clean readings.
    //    NOTE: this unloads ALL Ollama models, not just the AIrena model.
    //    If the user runs other Ollama consumers concurrently, their models will be evicted.
    if let Err(e) = client.unload_all_models().await {
        tracing::warn!("Failed to unload models during init: {e}");
    }

    // 5. Wait for GPU VRAM to settle
    tokio::time::sleep(Duration::from_millis(constants::OLLAMA_VRAM_SETTLE_MS)).await;

    // 6. Detect VRAM + recommend num_ctx (clean VRAM, loaded_vram_bytes = 0)
    let info =
        model_info::build_model_budget_info(&show, llm_size + emb_size, 0).await;

    // 7. Determine preload num_ctx: user's saved value if > 0, else recommendation.
    //    Without this, Ollama uses the model's native context length (often 128K),
    //    which can consume all VRAM just for the KV cache.
    let preload_num_ctx = if settings.num_ctx > 0 {
        Some(settings.num_ctx)
    } else {
        info.recommended_num_ctx
    };
    tracing::info!(
        "Preloading {llm_model} with num_ctx={preload_num_ctx:?} (saved={}, recommended={:?})",
        settings.num_ctx,
        info.recommended_num_ctx,
    );

    // 8. Preload LLM model with explicit num_ctx
    if let Err(e) = client.preload_model(preload_num_ctx).await {
        tracing::warn!("Failed to preload LLM model during init: {e}");
    }

    // 9. Preload embedding model if different from LLM (no num_ctx needed — embeddings don't use KV cache)
    if !embedding_model.is_empty() && embedding_model != llm_model {
        let emb_client = OllamaClient::new(&ollama_url, &embedding_model);
        if let Err(e) = emb_client.preload_model(None).await {
            tracing::warn!("Failed to preload embedding model during init: {e}");
        }
    }

    Ok(info)
}

#[tauri::command]
pub async fn compute_token_budget(
    params: BudgetParams,
    priorities: Vec<SectionPriority>,
) -> Result<TokenBudgetPreview, CommandError> {
    // Frontend priorities may have floor/ceiling=0 — apply default constants.
    let resolved = if priorities.is_empty() {
        token_budget::default_priorities()
    } else {
        token_budget::apply_default_bounds(&priorities)
    };
    Ok(TokenBudget::to_preview(&params, &resolved))
}
