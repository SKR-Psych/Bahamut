pub mod commands;
pub mod database;

use std::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub project_root: Mutex<Option<std::path::PathBuf>>,
    pub db_conn: Mutex<Option<rusqlite::Connection>>,
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

            let conn = database::initialize_db(app_data_dir)
                .expect("Failed to initialize SQLite database");

            app.manage(AppState {
                project_root: Mutex::new(None),
                db_conn: Mutex::new(Some(conn)),
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
            commands::system::get_hardware_info,
            commands::system::check_ollama_status,
            database::get_audit_logs,
            database::verify_audit_chain,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
