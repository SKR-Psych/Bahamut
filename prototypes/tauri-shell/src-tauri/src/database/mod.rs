use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;
use crate::AppState;

pub fn initialize_db(app_data_dir: PathBuf) -> Result<Connection, String> {
    // Ensure the folder exists
    fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create AppData directory: {}", e))?;

    let db_path = app_data_dir.join("bahamut.db");
    let conn = Connection::open(&db_path)
        .map_err(|e| format!("Failed to open SQLite database: {}", e))?;

    // Create settings table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT
        )",
        [],
    ).map_err(|e| format!("Failed to create settings table: {}", e))?;

    // Create audit_logs table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS audit_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
            action_type TEXT NOT NULL,
            details TEXT,
            status TEXT NOT NULL,
            error TEXT
        )",
        [],
    ).map_err(|e| format!("Failed to create audit_logs table: {}", e))?;

    println!("SQLite database initialized successfully at {:?}", db_path);
    Ok(conn)
}

pub fn log_action(
    state: &AppState,
    action_type: &str,
    details: Option<String>,
    status: &str,
    error: Option<String>,
) -> Result<(), String> {
    let conn_guard = state.db_conn.lock().map_err(|_| "Failed to lock database mutex")?;
    if let Some(conn) = &*conn_guard {
        conn.execute(
            "INSERT INTO audit_logs (action_type, details, status, error) VALUES (?1, ?2, ?3, ?4)",
            (action_type, details, status, error),
        ).map_err(|e| format!("Failed to insert audit log: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_audit_logs(state: tauri::State<'_, AppState>) -> Result<Vec<serde_json::Value>, String> {
    let conn_guard = state.db_conn.lock().map_err(|_| "Failed to lock database mutex")?;
    let conn = match &*conn_guard {
        Some(c) => c,
        None => return Err("Database connection not initialized".to_string()),
    };

    let mut stmt = conn.prepare("SELECT id, timestamp, action_type, details, status, error FROM audit_logs ORDER BY timestamp DESC LIMIT 100")
        .map_err(|e| e.to_string())?;

    let rows = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "timestamp": row.get::<_, String>(1)?,
            "action_type": row.get::<_, String>(2)?,
            "details": row.get::<_, Option<String>>(3)?,
            "status": row.get::<_, String>(4)?,
            "error": row.get::<_, Option<String>>(5)?,
        }))
    }).map_err(|e| e.to_string())?;

    let mut logs = Vec::new();
    for r in rows {
        if let Ok(l) = r {
            logs.push(l);
        }
    }
    Ok(logs)
}
