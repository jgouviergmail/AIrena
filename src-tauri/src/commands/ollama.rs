use tauri::State;

use crate::error::CommandError;
use crate::ollama::client::OllamaClient;
use crate::ollama::types::ModelInfo;
use crate::state::AppState;

#[tauri::command]
pub async fn check_ollama_connection(state: State<'_, AppState>) -> Result<bool, CommandError> {
    let settings = {
        let db = state.db.clone();
        crate::db::repository::get_settings(&db)
            .await
            .map_err(|e| CommandError::Settings(e.to_string()))?
    };
    let client = OllamaClient::new(&settings.ollama_url, "");
    Ok(client.check_connection().await)
}

#[tauri::command]
pub async fn list_ollama_models(state: State<'_, AppState>) -> Result<Vec<ModelInfo>, CommandError> {
    let settings = {
        let db = state.db.clone();
        crate::db::repository::get_settings(&db)
            .await
            .map_err(|e| CommandError::Settings(e.to_string()))?
    };
    let client = OllamaClient::new(&settings.ollama_url, "");
    client
        .list_models()
        .await
        .map_err(|e| CommandError::Ollama(e.to_string()))
}

#[tauri::command]
pub async fn preload_ollama_model(
    model: String,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    let settings = {
        let db = state.db.clone();
        crate::db::repository::get_settings(&db)
            .await
            .map_err(|e| CommandError::Settings(e.to_string()))?
    };
    let client = OllamaClient::new(&settings.ollama_url, &model);
    client
        .preload_model()
        .await
        .map_err(|e| CommandError::Ollama(e.to_string()))
}
