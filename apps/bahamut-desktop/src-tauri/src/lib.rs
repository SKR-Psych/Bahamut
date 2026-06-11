pub mod catalog;
pub mod commands;
pub mod context;
pub mod database;
pub mod providers;

use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};
use tauri::Manager;

pub struct AppState {
    pub project_root: Mutex<Option<std::path::PathBuf>>,
    pub db_conn: Mutex<Option<rusqlite::Connection>>,
    /// Application data directory (database, trash for recoverable deletes).
    pub app_data_dir: std::path::PathBuf,
    /// Generation counter for project search cancellation: a running search
    /// aborts as soon as the counter moves past the value it started with.
    pub search_generation: Arc<AtomicU64>,
    pub pull_generation: Arc<AtomicU64>,
    pub chat_generation: Arc<AtomicU64>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Get AppData directory for local settings and databases
            let app_data_dir = app.path().app_data_dir().unwrap_or_else(|_| {
                std::env::current_dir()
                    .unwrap_or_default()
                    .join("bahamut_data")
            });

            let conn = database::initialize_db(app_data_dir.clone())
                .expect("Failed to initialize SQLite database");

            app.manage(AppState {
                project_root: Mutex::new(None),
                db_conn: Mutex::new(Some(conn)),
                app_data_dir,
                search_generation: Arc::new(AtomicU64::new(0)),
                pull_generation: Arc::new(AtomicU64::new(0)),
                chat_generation: Arc::new(AtomicU64::new(0)),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::security::set_project_root,
            commands::security::check_file_in_sandbox,
            commands::files::list_project_files,
            commands::files::read_project_file,
            commands::files::save_project_file,
            commands::files::rollback_file_snapshot,
            commands::files::list_file_snapshots,
            commands::files::get_snapshot_content,
            commands::fileops::create_project_file,
            commands::fileops::create_project_folder,
            commands::fileops::rename_project_path,
            commands::fileops::delete_project_path,
            commands::search::search_project,
            commands::search::cancel_project_search,
            commands::settings::get_app_settings,
            commands::settings::update_app_settings,
            commands::settings::reset_app_settings,
            commands::system::get_hardware_info,
            commands::system::check_ollama_status,
            commands::ai::get_hardware_profile,
            commands::ai::get_model_catalogue,
            commands::ai::get_model_recommendations,
            commands::ai::get_provider_status,
            commands::ai::reconnect_provider,
            commands::ai::list_installed_models,
            commands::ai::get_installed_model_metadata,
            commands::ai::pull_model,
            commands::ai::cancel_model_pull,
            commands::ai::delete_model,
            commands::ai::select_active_model,
            commands::ai::test_prompt,
            commands::ai::start_chat,
            commands::ai::cancel_chat,
            commands::ai::assemble_chat_context,
            commands::ai::approve_secret_context,
            commands::ai::create_conversation,
            commands::ai::list_conversations,
            commands::ai::read_conversation,
            commands::ai::rename_conversation,
            commands::ai::delete_conversation,
            commands::ai::clear_conversation_history,
            commands::ai::inspect_stored_chat_data,
            database::get_audit_logs,
            database::verify_audit_chain,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
