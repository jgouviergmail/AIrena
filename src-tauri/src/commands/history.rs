use tauri::State;

use crate::db::repository;
use crate::error::CommandError;
use crate::models::history::{DiscussionDetail, DiscussionSummary, SaveDiscussionRequest};
use crate::state::AppState;

#[tauri::command]
pub async fn save_discussion_history(
    request: SaveDiscussionRequest,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    let db = state.db.clone();
    repository::save_discussion(&db, request)
        .await
        .map_err(|e| CommandError::History(e.to_string()))
}

#[tauri::command]
pub async fn list_discussion_history(
    state: State<'_, AppState>,
) -> Result<Vec<DiscussionSummary>, CommandError> {
    let db = state.db.clone();
    repository::list_discussions(&db)
        .await
        .map_err(|e| CommandError::History(e.to_string()))
}

#[tauri::command]
pub async fn get_discussion_history(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<DiscussionDetail>, CommandError> {
    let db = state.db.clone();
    repository::get_discussion(&db, &id)
        .await
        .map_err(|e| CommandError::History(e.to_string()))
}

#[tauri::command]
pub async fn delete_discussion_history(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    let db = state.db.clone();
    repository::delete_discussion(&db, &id)
        .await
        .map_err(|e| CommandError::History(e.to_string()))
}

#[tauri::command]
pub async fn delete_all_discussion_history(
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    let db = state.db.clone();
    repository::delete_all_discussions(&db)
        .await
        .map_err(|e| CommandError::History(e.to_string()))
}
