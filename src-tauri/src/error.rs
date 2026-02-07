use serde::Serialize;

/// Errors for Tauri commands — implements Into<InvokeError> via Serialize
#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "message")]
pub enum CommandError {
    #[error("Ollama error: {0}")]
    Ollama(String),
    #[error("Discussion error: {0}")]
    Discussion(String),
    #[error("Settings error: {0}")]
    Settings(String),
    #[error("Discussion already running")]
    AlreadyRunning,
    #[error("No active discussion")]
    NoActiveDiscussion,
}
