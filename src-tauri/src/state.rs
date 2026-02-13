use std::sync::{Arc, Mutex, MutexGuard};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::error::CommandError;
use crate::models::engine_command::EngineCommand;
use crate::models::settings::AppSettings;
use crate::rag::RagStore;

/// Global application state managed by Tauri
/// Uses std::sync::Mutex (NOT tokio::sync::Mutex) because the lock
/// is never held across an .await point.
/// Arc wrappers allow sharing with spawned tasks for cleanup.
pub struct AppState {
    pub engine_cmd_tx: Arc<Mutex<Option<mpsc::Sender<EngineCommand>>>>,
    pub cancel_token: Arc<Mutex<Option<CancellationToken>>>,
    pub db: tokio_rusqlite::Connection,
    /// In-memory RAG store (populated during setup, taken by engine at discussion start)
    pub rag_store: Arc<Mutex<Option<RagStore>>>,
}

impl AppState {
    /// Lock a std::sync::Mutex, recovering from poison.
    /// Safe because AppState fields are always left in a consistent state.
    pub fn lock_or_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
        mutex.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Clone the engine command sender from the mutex-protected slot.
    pub async fn send_engine_command(&self, cmd: EngineCommand) -> Result<(), CommandError> {
        let tx = Self::lock_or_recover(&self.engine_cmd_tx).clone();
        match tx {
            Some(tx) => tx
                .send(cmd)
                .await
                .map_err(|_| CommandError::NoActiveDiscussion),
            None => Err(CommandError::NoActiveDiscussion),
        }
    }

    /// Read app settings from the database.
    pub async fn get_settings(&self) -> Result<AppSettings, CommandError> {
        let db = self.db.clone();
        crate::db::repository::get_settings(&db)
            .await
            .map_err(|e| CommandError::Settings(e.to_string()))
    }

    /// Clear the engine command sender and cancel token slots.
    /// Used for cleanup after engine ends or on startup failure rollback.
    pub fn clear_engine_slots(
        tx: &Arc<Mutex<Option<mpsc::Sender<EngineCommand>>>>,
        cancel: &Arc<Mutex<Option<CancellationToken>>>,
    ) {
        *Self::lock_or_recover(tx) = None;
        *Self::lock_or_recover(cancel) = None;
    }
}
