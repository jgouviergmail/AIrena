use tauri::State;

use crate::db::repository;
use crate::error::CommandError;
use crate::license;
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
pub async fn list_arbitre_profiles(state: State<'_, AppState>) -> Result<Vec<PredefinedProfile>, CommandError> {
    let db = state.db.clone();
    repository::list_arbitre_profiles(&db)
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

// ── License commands ────────────────────────────────────────────────

#[tauri::command]
pub async fn validate_license_key(
    key: String,
    state: State<'_, AppState>,
) -> Result<license::LicenseStatus, CommandError> {
    let db = state.db.clone();
    let (stored_hash, disc_count, last_check) = repository::get_license_tracking(&db)
        .await
        .map_err(|e| CommandError::License(e.to_string()))?;
    let status = license::check_license_status(&key, &stored_hash, disc_count, last_check);
    // Update timestamp (anti-clock, best-effort)
    let _ = repository::update_last_check_timestamp(&db).await;
    Ok(status)
}

#[tauri::command]
pub async fn check_license_status(
    state: State<'_, AppState>,
) -> Result<license::LicenseStatus, CommandError> {
    let settings = state.get_settings().await?;
    if settings.license_key.is_empty() {
        return Ok(license::LicenseStatus::invalid("No license key configured"));
    }
    let db = state.db.clone();
    let (stored_hash, disc_count, last_check) = repository::get_license_tracking(&db)
        .await
        .map_err(|e| CommandError::License(e.to_string()))?;
    let status = license::check_license_status(&settings.license_key, &stored_hash, disc_count, last_check);
    let _ = repository::update_last_check_timestamp(&db).await;
    Ok(status)
}
