//! Narrow settings commands. Settings that affect backend security limits
//! (file-size caps) and persisted UI preferences live in the SQLite settings
//! table behind validation — the frontend cannot write arbitrary keys, and
//! credentials are never stored here (the future credential store uses the
//! OS keychain per docs/architecture.md).

use crate::commands::files::with_root_and_conn_optional;
use crate::database;
use crate::AppState;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::State;

/// Validation bounds for size limits: 1 KiB .. 50 MiB.
const MIN_SIZE_BYTES: u64 = 1024;
const MAX_SIZE_BYTES: u64 = 50 * 1024 * 1024;

const ALLOWED_THEMES: [&str; 1] = ["dark"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiPrefs {
    #[serde(default = "default_true")]
    pub glassmorphism: bool,
    #[serde(default)]
    pub solid_mode: bool,
    #[serde(default = "default_true")]
    pub confirm_tab_close: bool,
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_true() -> bool {
    true
}
fn default_theme() -> String {
    "dark".to_string()
}

impl Default for UiPrefs {
    fn default() -> Self {
        UiPrefs {
            glassmorphism: true,
            solid_mode: false,
            confirm_tab_close: true,
            theme: default_theme(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppSettings {
    pub max_file_size_bytes: u64,
    pub max_search_file_size_bytes: u64,
    pub ui_prefs: UiPrefs,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            max_file_size_bytes: database::DEFAULT_MAX_FILE_SIZE,
            max_search_file_size_bytes: database::DEFAULT_MAX_SEARCH_FILE_SIZE,
            ui_prefs: UiPrefs::default(),
        }
    }
}

pub fn get_settings_core(conn: &Connection) -> Result<AppSettings, String> {
    let ui_prefs = database::get_setting(conn, database::UI_PREFS_KEY)?
        .and_then(|raw| serde_json::from_str::<UiPrefs>(&raw).ok())
        .unwrap_or_default();
    Ok(AppSettings {
        max_file_size_bytes: database::get_max_file_size(conn),
        max_search_file_size_bytes: database::get_max_search_file_size(conn),
        ui_prefs,
    })
}

fn validate(settings: &AppSettings) -> Result<(), String> {
    for (label, value) in [
        ("Maximum editable file size", settings.max_file_size_bytes),
        (
            "Maximum searched file size",
            settings.max_search_file_size_bytes,
        ),
    ] {
        if !(MIN_SIZE_BYTES..=MAX_SIZE_BYTES).contains(&value) {
            return Err(format!(
                "{} must be between {} and {} bytes",
                label, MIN_SIZE_BYTES, MAX_SIZE_BYTES
            ));
        }
    }
    if !ALLOWED_THEMES.contains(&settings.ui_prefs.theme.as_str()) {
        return Err(format!(
            "Unknown theme '{}' (allowed: {})",
            settings.ui_prefs.theme,
            ALLOWED_THEMES.join(", ")
        ));
    }
    Ok(())
}

pub fn update_settings_core(conn: &Connection, settings: &AppSettings) -> Result<(), String> {
    validate(settings)?;
    database::set_setting(
        conn,
        database::MAX_FILE_SIZE_KEY,
        &settings.max_file_size_bytes.to_string(),
    )?;
    database::set_setting(
        conn,
        database::MAX_SEARCH_FILE_SIZE_KEY,
        &settings.max_search_file_size_bytes.to_string(),
    )?;
    let prefs_json = serde_json::to_string(&settings.ui_prefs)
        .map_err(|e| format!("Failed to serialize UI preferences: {}", e))?;
    database::set_setting(conn, database::UI_PREFS_KEY, &prefs_json)?;

    database::log_action_with_conn(
        conn,
        "update_settings",
        Some(
            serde_json::json!({
                "max_file_size_bytes": settings.max_file_size_bytes,
                "max_search_file_size_bytes": settings.max_search_file_size_bytes,
                "ui_prefs": settings.ui_prefs,
            })
            .to_string(),
        ),
        "success",
        None,
    )
}

pub fn reset_settings_core(conn: &Connection) -> Result<AppSettings, String> {
    database::delete_setting(conn, database::MAX_FILE_SIZE_KEY)?;
    database::delete_setting(conn, database::MAX_SEARCH_FILE_SIZE_KEY)?;
    database::delete_setting(conn, database::UI_PREFS_KEY)?;
    database::log_action_with_conn(conn, "reset_settings", None, "success", None)?;
    Ok(AppSettings::default())
}

// ---------------------------------------------------------------------------
// Tauri command wrappers (settings work with or without an open project)
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_app_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    with_root_and_conn_optional(&state, |_root, conn| get_settings_core(conn))
}

#[tauri::command]
pub fn update_app_settings(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    with_root_and_conn_optional(&state, |_root, conn| {
        update_settings_core(conn, &settings)?;
        get_settings_core(conn)
    })
}

#[tauri::command]
pub fn reset_app_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    with_root_and_conn_optional(&state, |_root, conn| reset_settings_core(conn))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::init_schema;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        conn
    }

    fn audit_rows(conn: &Connection, action: &str) -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM audit_logs WHERE action_type = ?1 AND status = 'success'",
            rusqlite::params![action],
            |row| row.get(0),
        )
        .unwrap()
    }

    #[test]
    fn returns_safe_defaults_when_unset() {
        let conn = test_conn();
        let s = get_settings_core(&conn).unwrap();
        assert_eq!(s.max_file_size_bytes, database::DEFAULT_MAX_FILE_SIZE);
        assert_eq!(
            s.max_search_file_size_bytes,
            database::DEFAULT_MAX_SEARCH_FILE_SIZE
        );
        assert!(s.ui_prefs.glassmorphism);
        assert!(!s.ui_prefs.solid_mode);
        assert!(s.ui_prefs.confirm_tab_close);
        assert_eq!(s.ui_prefs.theme, "dark");
    }

    #[test]
    fn update_persists_and_audits() {
        let conn = test_conn();
        let mut s = AppSettings::default();
        s.max_file_size_bytes = 4 * 1024 * 1024;
        s.ui_prefs.solid_mode = true;
        s.ui_prefs.confirm_tab_close = false;
        update_settings_core(&conn, &s).unwrap();

        let loaded = get_settings_core(&conn).unwrap();
        assert_eq!(loaded.max_file_size_bytes, 4 * 1024 * 1024);
        assert!(loaded.ui_prefs.solid_mode);
        assert!(!loaded.ui_prefs.confirm_tab_close);
        assert_eq!(audit_rows(&conn, "update_settings"), 1);
    }

    #[test]
    fn rejects_out_of_range_sizes_and_unknown_theme() {
        let conn = test_conn();
        let mut s = AppSettings::default();
        s.max_file_size_bytes = 0;
        assert!(update_settings_core(&conn, &s)
            .unwrap_err()
            .contains("between"));

        let mut s = AppSettings::default();
        s.max_search_file_size_bytes = 500 * 1024 * 1024;
        assert!(update_settings_core(&conn, &s)
            .unwrap_err()
            .contains("between"));

        let mut s = AppSettings::default();
        s.ui_prefs.theme = "neon".to_string();
        assert!(update_settings_core(&conn, &s)
            .unwrap_err()
            .contains("Unknown theme"));

        // Nothing was persisted by the failed updates.
        let loaded = get_settings_core(&conn).unwrap();
        assert_eq!(loaded.max_file_size_bytes, database::DEFAULT_MAX_FILE_SIZE);
    }

    #[test]
    fn reset_restores_defaults() {
        let conn = test_conn();
        let mut s = AppSettings::default();
        s.max_file_size_bytes = 8 * 1024 * 1024;
        update_settings_core(&conn, &s).unwrap();

        let restored = reset_settings_core(&conn).unwrap();
        assert_eq!(
            restored.max_file_size_bytes,
            database::DEFAULT_MAX_FILE_SIZE
        );
        let loaded = get_settings_core(&conn).unwrap();
        assert_eq!(loaded.max_file_size_bytes, database::DEFAULT_MAX_FILE_SIZE);
        assert_eq!(audit_rows(&conn, "reset_settings"), 1);
    }
}
