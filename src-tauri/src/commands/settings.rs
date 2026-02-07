use tauri::State;

use crate::db::repository;
use crate::error::CommandError;
use crate::models::profile::PredefinedProfile;
use crate::models::settings::AppSettings;
use crate::state::AppState;

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, CommandError> {
    let db = state.db.clone();
    repository::get_settings(&db)
        .await
        .map_err(|e| CommandError::Settings(e.to_string()))
}

#[tauri::command]
pub async fn save_settings(
    settings: AppSettings,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    if settings.username.trim().is_empty() {
        return Err(CommandError::Settings("Username must not be empty".to_string()));
    }
    let db = state.db.clone();
    repository::save_settings(&db, &settings)
        .await
        .map_err(|e| CommandError::Settings(e.to_string()))
}

#[tauri::command]
pub async fn list_profiles(state: State<'_, AppState>) -> Result<Vec<PredefinedProfile>, CommandError> {
    let db = state.db.clone();
    repository::list_profiles(&db)
        .await
        .map_err(|e| CommandError::Settings(e.to_string()))
}

#[tauri::command]
pub async fn get_profile(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<PredefinedProfile>, CommandError> {
    let db = state.db.clone();
    repository::get_profile(&db, &id)
        .await
        .map_err(|e| CommandError::Settings(e.to_string()))
}

#[tauri::command]
pub async fn save_profile(
    profile: PredefinedProfile,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    let db = state.db.clone();
    repository::save_profile(&db, &profile)
        .await
        .map_err(|e| CommandError::Settings(e.to_string()))
}

#[tauri::command]
pub async fn delete_profile(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    let db = state.db.clone();
    repository::delete_profile(&db, &id)
        .await
        .map_err(|e| CommandError::Settings(e.to_string()))
}
