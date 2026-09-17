//! Operations (v1.20): the backend log of the day for the "export the journal"
//! button, and the application version.

use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use crate::constants;
use crate::error::CommandError;

/// Directory of the rotating log files (next to the executable, as `lib.rs` sets it up).
pub fn log_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(constants::LOG_DIR_NAME)))
        .unwrap_or_else(|| PathBuf::from(constants::LOG_DIR_NAME))
}

/// The most recent log file of the directory (`airena.log.YYYY-MM-DD`), if any.
pub fn latest_log_file(dir: &std::path::Path) -> Option<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(constants::LOG_FILE_PREFIX)))
        .collect();
    files.sort();
    files.pop()
}

/// Last `max_bytes` of a text file, cut on a line boundary (UTF-8 safe). Only
/// the tail is read: a day of verbose logging may weigh tens of megabytes.
pub fn tail_of_file(path: &std::path::Path, max_bytes: usize) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let len = file.metadata()?.len();
    let start = len.saturating_sub(max_bytes as u64);
    file.seek(SeekFrom::Start(start))?;
    let mut bytes = Vec::with_capacity((len - start) as usize);
    file.read_to_end(&mut bytes)?;
    let mut text = String::from_utf8_lossy(&bytes).into_owned();
    if start > 0 {
        if let Some(nl) = text.find('\n') {
            text.drain(..=nl);
        }
    }
    Ok(text)
}

/// The tail of today's backend log (empty when no log exists yet). Never
/// contains secrets: API keys are masked before they reach the log.
#[tauri::command]
pub fn read_backend_log() -> Result<String, CommandError> {
    let dir = log_dir();
    let Some(file) = latest_log_file(&dir) else { return Ok(String::new()) };
    tail_of_file(&file, constants::LOG_EXPORT_MAX_BYTES).map_err(|e| CommandError::Settings(format!("Cannot read the log file: {e}")))
}

/// Version of the running application (from the crate manifest).
#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_latest_log_and_tails_it_on_a_line_boundary() {
        let dir = std::env::temp_dir().join(format!("airena-logs-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("airena.log.2026-09-15"), "old\n").unwrap();
        std::fs::write(dir.join("airena.log.2026-09-16"), "line one\nline two\nline three\n").unwrap();
        std::fs::write(dir.join("other.txt"), "x").unwrap();
        let latest = latest_log_file(&dir).unwrap();
        assert!(latest.ends_with("airena.log.2026-09-16"));
        assert_eq!(tail_of_file(&latest, 1000).unwrap(), "line one\nline two\nline three\n");
        // A cut in the middle of a line drops that partial line
        assert_eq!(tail_of_file(&latest, 15).unwrap(), "line three\n");
        assert!(latest_log_file(&dir.join("missing")).is_none());
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(!get_app_version().is_empty());
    }
}
