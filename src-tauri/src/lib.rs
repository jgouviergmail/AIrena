mod commands;
mod constants;
mod db;
mod engine;
mod error;
mod license;
mod models;
mod ollama;
mod rag;
mod state;
mod tavily;
mod wikipedia;

use std::sync::{Arc, Mutex};

use tauri::Manager;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Determine log directory: next to the executable in production
    let log_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("logs")))
        .unwrap_or_else(|| std::path::PathBuf::from("logs"));
    let _ = std::fs::create_dir_all(&log_dir);

    // File appender: daily rotation, keeps 7 days
    let file_appender = tracing_appender::rolling::daily(&log_dir, "airena.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Build subscriber with console + file layers
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("airena=info,airena_lib=info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().with_target(true).with_thread_ids(false))
        .with(
            fmt::layer()
                .with_target(true)
                .with_ansi(false)
                .with_writer(non_blocking),
        )
        .init();

    // Keep the _guard alive for the entire app lifetime by leaking it.
    // This ensures the file writer flushes on shutdown.
    let _guard = Box::leak(Box::new(_guard));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // Initialize SQLite database in app data directory
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data directory");
            std::fs::create_dir_all(&app_data_dir)
                .expect("Failed to create app data directory");

            let db_path = app_data_dir.join("airena.db");
            let db = tauri::async_runtime::block_on(async {
                let conn = tokio_rusqlite::Connection::open(db_path)
                    .await
                    .expect("Failed to open database");
                db::schema::initialize(&conn)
                    .await
                    .expect("Failed to initialize database schema");
                db::seed::seed_profiles(&conn)
                    .await
                    .expect("Failed to seed profiles");
                conn
            });

            let state = AppState {
                engine_cmd_tx: Arc::new(Mutex::new(None)),
                cancel_token: Arc::new(Mutex::new(None)),
                db,
                rag_store: Arc::new(Mutex::new(None)),
            };
            app.manage(state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Discussion commands
            commands::discussion::start_discussion,
            commands::discussion::pause_discussion,
            commands::discussion::resume_discussion,
            commands::discussion::stop_discussion,
            commands::discussion::force_stop_discussion,
            commands::discussion::user_wants_to_intervene,
            commands::discussion::submit_user_message,
            commands::discussion::skip_user_turn,
            commands::discussion::adjust_emotion,
            // Ollama commands
            commands::ollama::check_ollama_connection,
            commands::ollama::list_ollama_models,
            commands::ollama::preload_ollama_model,
            commands::ollama::get_model_budget_info,
            commands::ollama::initialize_ollama,
            commands::ollama::compute_token_budget,
            // Settings commands
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::list_profiles,
            commands::settings::list_arbitre_profiles,
            commands::settings::get_profile,
            commands::settings::save_profile,
            commands::settings::delete_profile,
            // License commands
            commands::settings::validate_license_key,
            commands::settings::check_license_status,
            // History commands
            commands::history::save_discussion_history,
            commands::history::list_discussion_history,
            commands::history::get_discussion_history,
            commands::history::delete_discussion_history,
            commands::history::delete_all_discussion_history,
            // RAG commands
            commands::rag::import_rag_document,
            commands::rag::remove_rag_document,
            commands::rag::get_rag_status,
            commands::rag::clear_rag_store,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
