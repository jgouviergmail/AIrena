use std::sync::{Arc, Mutex, MutexGuard};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::error::CommandError;
use crate::models::engine_command::EngineCommand;

/// Global application state managed by Tauri
/// Uses std::sync::Mutex (NOT tokio::sync::Mutex) because the lock
/// is never held across an .await point.
/// Arc wrappers allow sharing with spawned tasks for cleanup.
pub struct AppState {
    pub engine_cmd_tx: Arc<Mutex<Option<mpsc::Sender<EngineCommand>>>>,
    pub cancel_token: Arc<Mutex<Option<CancellationToken>>>,
    pub db: tokio_rusqlite::Connection,
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
}
