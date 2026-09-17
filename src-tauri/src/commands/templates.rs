//! Discussion templates (v1.20): list, save, delete. The configuration is an
//! opaque JSON object owned by the frontend.

use tauri::State;

use crate::db::repository;
use crate::error::CommandError;
use crate::models::template::DiscussionTemplate;
use crate::state::AppState;

#[tauri::command]
pub async fn list_discussion_templates(state: State<'_, AppState>) -> Result<Vec<DiscussionTemplate>, CommandError> {
    let db = state.db.clone();
    repository::list_templates(&db).await.map_err(|e| CommandError::History(e.to_string()))
}

/// Save a user template (never marks it builtin, whatever the payload says).
#[tauri::command]
pub async fn save_discussion_template(template: DiscussionTemplate, state: State<'_, AppState>) -> Result<(), CommandError> {
    template.validate().map_err(CommandError::Settings)?;
    let db = state.db.clone();
    let template = DiscussionTemplate {
        builtin: false,
        created_at: if template.created_at.trim().is_empty() { chrono::Utc::now().to_rfc3339() } else { template.created_at },
        ..template
    };
    repository::save_template(&db, template).await.map_err(|e| CommandError::History(e.to_string()))
}

/// Delete a user template; a builtin one is refused.
#[tauri::command]
pub async fn delete_discussion_template(id: String, state: State<'_, AppState>) -> Result<(), CommandError> {
    let db = state.db.clone();
    let deleted = repository::delete_template(&db, &id).await.map_err(|e| CommandError::History(e.to_string()))?;
    if !deleted {
        return Err(CommandError::Settings("Builtin templates cannot be deleted".to_string()));
    }
    Ok(())
}
