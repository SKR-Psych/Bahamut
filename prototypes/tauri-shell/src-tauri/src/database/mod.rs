use crate::AppState;
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

/// Fixed value the first audit entry chains from.
pub const GENESIS_HASH: &str = "bahamut-audit-genesis-v1";

/// Settings key mirroring the hash of the newest audit entry, so deletion of
/// trailing rows (which would otherwise leave a valid shorter chain) is
/// detectable.
const CHAIN_HEAD_KEY: &str = "audit_chain_head";

const AUDIT_TABLE_COLUMNS: &str = "(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    seq INTEGER NOT NULL UNIQUE,
    timestamp DATETIME NOT NULL,
    action_type TEXT NOT NULL,
    details TEXT,
    status TEXT NOT NULL,
    error TEXT,
    prev_hash TEXT NOT NULL,
    entry_hash TEXT NOT NULL
)";

pub fn initialize_db(app_data_dir: PathBuf) -> Result<Connection, String> {
    // Ensure the folder exists
    fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create AppData directory: {}", e))?;

    let db_path = app_data_dir.join("bahamut.db");
    let conn =
        Connection::open(&db_path).map_err(|e| format!("Failed to open SQLite database: {}", e))?;

    init_schema(&conn)?;

    println!("SQLite database initialized successfully at {:?}", db_path);
    Ok(conn)
}

/// Creates the settings and audit tables; migrates a legacy (pre-hash-chain)
/// audit_logs table in place, back-filling the chain over existing rows.
pub fn init_schema(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT
        )",
        [],
    )
    .map_err(|e| format!("Failed to create settings table: {}", e))?;

    let audit_table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'audit_logs'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count > 0)
        .map_err(|e| format!("Failed to inspect database schema: {}", e))?;

    if audit_table_exists && !column_exists(conn, "audit_logs", "entry_hash")? {
        migrate_legacy_audit_table(conn)?;
    } else {
        conn.execute(
            &format!(
                "CREATE TABLE IF NOT EXISTS audit_logs {}",
                AUDIT_TABLE_COLUMNS
            ),
            [],
        )
        .map_err(|e| format!("Failed to create audit_logs table: {}", e))?;
    }

    Ok(())
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool, String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({})", table))
        .map_err(|e| format!("Failed to inspect table {}: {}", table, e))?;
    let names = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| format!("Failed to read columns of {}: {}", table, e))?;
    for name in names.flatten() {
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Rebuilds a legacy audit_logs table (no seq/hash columns) into the
/// hash-chained schema, preserving all rows in id order.
fn migrate_legacy_audit_table(conn: &Connection) -> Result<(), String> {
    conn.execute("BEGIN IMMEDIATE", [])
        .map_err(|e| format!("Failed to begin migration transaction: {}", e))?;

    let result = (|| -> Result<(), String> {
        conn.execute(
            &format!("CREATE TABLE audit_logs_new {}", AUDIT_TABLE_COLUMNS),
            [],
        )
        .map_err(|e| format!("Failed to create migrated audit table: {}", e))?;

        type LegacyRow = (String, String, Option<String>, String, Option<String>);
        let mut stmt = conn
            .prepare(
                "SELECT timestamp, action_type, details, status, error
                 FROM audit_logs ORDER BY id ASC",
            )
            .map_err(|e| format!("Failed to read legacy audit rows: {}", e))?;
        let rows: Vec<LegacyRow> = stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })
            .map_err(|e| format!("Failed to read legacy audit rows: {}", e))?
            .collect::<Result<_, _>>()
            .map_err(|e| format!("Failed to read legacy audit rows: {}", e))?;
        drop(stmt);

        let mut prev_hash = GENESIS_HASH.to_string();
        let mut seq: i64 = 1;
        for (timestamp, action_type, details, status, error) in rows {
            let payload = canonical_payload(
                &timestamp,
                &action_type,
                details.as_deref(),
                &status,
                error.as_deref(),
            );
            let entry_hash = compute_entry_hash(seq, &prev_hash, &payload);
            conn.execute(
                "INSERT INTO audit_logs_new
                    (seq, timestamp, action_type, details, status, error, prev_hash, entry_hash)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    seq,
                    timestamp,
                    action_type,
                    details,
                    status,
                    error,
                    prev_hash,
                    entry_hash
                ],
            )
            .map_err(|e| format!("Failed to migrate audit row {}: {}", seq, e))?;
            prev_hash = entry_hash;
            seq += 1;
        }

        conn.execute("DROP TABLE audit_logs", [])
            .map_err(|e| format!("Failed to drop legacy audit table: {}", e))?;
        conn.execute("ALTER TABLE audit_logs_new RENAME TO audit_logs", [])
            .map_err(|e| format!("Failed to rename migrated audit table: {}", e))?;
        if seq > 1 {
            conn.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                params![CHAIN_HEAD_KEY, prev_hash],
            )
            .map_err(|e| format!("Failed to record chain head: {}", e))?;
        }
        Ok(())
    })();

    match result {
        Ok(()) => conn
            .execute("COMMIT", [])
            .map(|_| ())
            .map_err(|e| format!("Failed to commit migration: {}", e)),
        Err(e) => {
            let _ = conn.execute("ROLLBACK", []);
            Err(e)
        }
    }
}

/// Canonical serialization of an entry payload. serde_json's default map is
/// a BTreeMap, so keys serialize in sorted order and the output is
/// deterministic for identical values.
fn canonical_payload(
    timestamp: &str,
    action_type: &str,
    details: Option<&str>,
    status: &str,
    error: Option<&str>,
) -> String {
    serde_json::json!({
        "action_type": action_type,
        "details": details,
        "error": error,
        "status": status,
        "timestamp": timestamp,
    })
    .to_string()
}

/// entry_hash = SHA-256(seq || prev_hash || canonical payload), '|'-separated
/// to keep field boundaries unambiguous, hex-encoded.
fn compute_entry_hash(seq: i64, prev_hash: &str, canonical_payload: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(seq.to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(prev_hash.as_bytes());
    hasher.update(b"|");
    hasher.update(canonical_payload.as_bytes());
    let digest = hasher.finalize();

    use std::fmt::Write as _;
    let mut hex = String::with_capacity(64);
    for byte in digest {
        let _ = write!(hex, "{:02x}", byte);
    }
    hex
}

pub fn log_action(
    state: &AppState,
    action_type: &str,
    details: Option<String>,
    status: &str,
    error: Option<String>,
) -> Result<(), String> {
    let conn_guard = state
        .db_conn
        .lock()
        .map_err(|_| "Failed to lock database mutex")?;
    if let Some(conn) = &*conn_guard {
        log_action_with_conn(conn, action_type, details, status, error)?;
    }
    Ok(())
}

/// Appends a hash-chained entry. The previous-entry read, the insert, and the
/// chain-head update happen in one IMMEDIATE transaction so concurrent writers
/// cannot interleave and break the chain.
pub fn log_action_with_conn(
    conn: &Connection,
    action_type: &str,
    details: Option<String>,
    status: &str,
    error: Option<String>,
) -> Result<(), String> {
    conn.execute("BEGIN IMMEDIATE", [])
        .map_err(|e| format!("Failed to begin audit transaction: {}", e))?;

    let result = (|| -> Result<(), String> {
        let last: Option<(i64, String)> = conn
            .query_row(
                "SELECT seq, entry_hash FROM audit_logs ORDER BY seq DESC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|e| format!("Failed to read audit chain tail: {}", e))?;
        let (seq, prev_hash) = match last {
            Some((last_seq, last_hash)) => (last_seq + 1, last_hash),
            None => (1, GENESIS_HASH.to_string()),
        };

        // Generate the timestamp up front so the stored value is exactly what
        // the hash covers.
        let timestamp: String = conn
            .query_row("SELECT CURRENT_TIMESTAMP", [], |row| row.get(0))
            .map_err(|e| format!("Failed to read database timestamp: {}", e))?;

        let payload = canonical_payload(
            &timestamp,
            action_type,
            details.as_deref(),
            status,
            error.as_deref(),
        );
        let entry_hash = compute_entry_hash(seq, &prev_hash, &payload);

        conn.execute(
            "INSERT INTO audit_logs
                (seq, timestamp, action_type, details, status, error, prev_hash, entry_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                seq,
                timestamp,
                action_type,
                details,
                status,
                error,
                prev_hash,
                entry_hash
            ],
        )
        .map_err(|e| format!("Failed to insert audit log: {}", e))?;

        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![CHAIN_HEAD_KEY, entry_hash],
        )
        .map_err(|e| format!("Failed to update chain head: {}", e))?;
        Ok(())
    })();

    match result {
        Ok(()) => conn
            .execute("COMMIT", [])
            .map(|_| ())
            .map_err(|e| format!("Failed to commit audit entry: {}", e)),
        Err(e) => {
            let _ = conn.execute("ROLLBACK", []);
            Err(e)
        }
    }
}

/// Result of walking the audit hash chain.
#[derive(Debug, serde::Serialize)]
pub struct ChainVerification {
    pub valid: bool,
    pub entries_checked: i64,
    /// Sequence number of the first entry whose link is broken (for a
    /// deletion, the sequence number the missing entry should have had).
    pub first_broken_seq: Option<i64>,
    pub detail: Option<String>,
}

/// Walks the audit table in sequence order, recomputing every entry hash and
/// prev-hash link, then checks the recorded chain head so deletion of trailing
/// rows is also detected. Reports the first broken link found.
pub fn verify_chain(conn: &Connection) -> Result<ChainVerification, String> {
    type AuditRow = (
        i64,
        String,
        String,
        Option<String>,
        String,
        Option<String>,
        String,
        String,
    );
    let mut stmt = conn
        .prepare(
            "SELECT seq, timestamp, action_type, details, status, error, prev_hash, entry_hash
             FROM audit_logs ORDER BY seq ASC",
        )
        .map_err(|e| format!("Failed to read audit log: {}", e))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
            ))
        })
        .map_err(|e| format!("Failed to read audit log: {}", e))?;

    let broken = |checked: i64, seq: i64, detail: String| ChainVerification {
        valid: false,
        entries_checked: checked,
        first_broken_seq: Some(seq),
        detail: Some(detail),
    };

    let mut expected_seq: i64 = 1;
    let mut expected_prev = GENESIS_HASH.to_string();
    let mut checked: i64 = 0;

    for row in rows {
        let (seq, timestamp, action_type, details, status, error, prev_hash, entry_hash): AuditRow =
            row.map_err(|e| format!("Failed to read audit row: {}", e))?;

        if seq != expected_seq {
            return Ok(broken(
                checked,
                expected_seq,
                format!(
                    "sequence gap: expected seq {} but found {} (entry deleted?)",
                    expected_seq, seq
                ),
            ));
        }
        if prev_hash != expected_prev {
            return Ok(broken(
                checked,
                seq,
                format!("previous-hash link mismatch at seq {}", seq),
            ));
        }
        let payload = canonical_payload(
            &timestamp,
            &action_type,
            details.as_deref(),
            &status,
            error.as_deref(),
        );
        if compute_entry_hash(seq, &prev_hash, &payload) != entry_hash {
            return Ok(broken(
                checked,
                seq,
                format!("entry hash mismatch at seq {} (entry altered)", seq),
            ));
        }

        expected_prev = entry_hash;
        expected_seq += 1;
        checked += 1;
    }

    let head: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![CHAIN_HEAD_KEY],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("Failed to read chain head: {}", e))?;
    if let Some(head) = head {
        if checked == 0 {
            return Ok(broken(
                0,
                1,
                "chain head recorded but audit log is empty (all entries deleted?)".to_string(),
            ));
        }
        if head != expected_prev {
            return Ok(broken(
                checked,
                expected_seq,
                format!(
                    "chain head mismatch: log ends at seq {} but head points elsewhere (trailing entries deleted?)",
                    expected_seq - 1
                ),
            ));
        }
    }

    Ok(ChainVerification {
        valid: true,
        entries_checked: checked,
        first_broken_seq: None,
        detail: None,
    })
}

#[tauri::command]
pub fn verify_audit_chain(state: tauri::State<'_, AppState>) -> Result<ChainVerification, String> {
    let conn_guard = state
        .db_conn
        .lock()
        .map_err(|_| "Failed to lock database mutex")?;
    let conn = match &*conn_guard {
        Some(c) => c,
        None => return Err("Database connection not initialized".to_string()),
    };
    verify_chain(conn)
}

#[tauri::command]
pub fn get_audit_logs(state: tauri::State<'_, AppState>) -> Result<Vec<serde_json::Value>, String> {
    let conn_guard = state
        .db_conn
        .lock()
        .map_err(|_| "Failed to lock database mutex")?;
    let conn = match &*conn_guard {
        Some(c) => c,
        None => return Err("Database connection not initialized".to_string()),
    };

    let mut stmt = conn.prepare("SELECT id, seq, timestamp, action_type, details, status, error, entry_hash FROM audit_logs ORDER BY seq DESC LIMIT 100")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "seq": row.get::<_, i64>(1)?,
                "timestamp": row.get::<_, String>(2)?,
                "action_type": row.get::<_, String>(3)?,
                "details": row.get::<_, Option<String>>(4)?,
                "status": row.get::<_, String>(5)?,
                "error": row.get::<_, Option<String>>(6)?,
                "entry_hash": row.get::<_, String>(7)?,
            }))
        })
        .map_err(|e| e.to_string())?;

    let mut logs = Vec::new();
    for l in rows.flatten() {
        logs.push(l);
    }
    Ok(logs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        conn
    }

    fn log_entries(conn: &Connection, n: usize) {
        for i in 0..n {
            log_action_with_conn(
                conn,
                "test_action",
                Some(format!("detail {}", i)),
                "success",
                None,
            )
            .unwrap();
        }
    }

    #[test]
    fn empty_chain_verifies() {
        let conn = test_conn();
        let report = verify_chain(&conn).unwrap();
        assert!(report.valid);
        assert_eq!(report.entries_checked, 0);
    }

    #[test]
    fn intact_chain_verifies() {
        let conn = test_conn();
        log_entries(&conn, 5);
        let report = verify_chain(&conn).unwrap();
        assert!(report.valid, "{:?}", report);
        assert_eq!(report.entries_checked, 5);
        assert_eq!(report.first_broken_seq, None);
    }

    #[test]
    fn altered_entry_is_detected_at_its_row() {
        let conn = test_conn();
        log_entries(&conn, 5);
        conn.execute(
            "UPDATE audit_logs SET details = 'tampered' WHERE seq = 3",
            [],
        )
        .unwrap();
        let report = verify_chain(&conn).unwrap();
        assert!(!report.valid);
        assert_eq!(report.first_broken_seq, Some(3));
        assert!(report.detail.unwrap().contains("altered"));
    }

    #[test]
    fn altered_entry_with_recomputed_hash_breaks_next_link() {
        // An attacker who alters a row AND recomputes its entry_hash still
        // breaks the chain: the next row's prev_hash no longer matches.
        let conn = test_conn();
        log_entries(&conn, 5);
        let (timestamp, prev_hash): (String, String) = conn
            .query_row(
                "SELECT timestamp, prev_hash FROM audit_logs WHERE seq = 2",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let payload =
            canonical_payload(&timestamp, "test_action", Some("tampered"), "success", None);
        let forged_hash = compute_entry_hash(2, &prev_hash, &payload);
        conn.execute(
            "UPDATE audit_logs SET details = 'tampered', entry_hash = ?1 WHERE seq = 2",
            params![forged_hash],
        )
        .unwrap();

        let report = verify_chain(&conn).unwrap();
        assert!(!report.valid);
        assert_eq!(report.first_broken_seq, Some(3));
    }

    #[test]
    fn deleted_middle_entry_is_detected() {
        let conn = test_conn();
        log_entries(&conn, 5);
        conn.execute("DELETE FROM audit_logs WHERE seq = 3", [])
            .unwrap();
        let report = verify_chain(&conn).unwrap();
        assert!(!report.valid);
        assert_eq!(report.first_broken_seq, Some(3));
        assert!(report.detail.unwrap().contains("deleted"));
    }

    #[test]
    fn deleted_tail_entry_is_detected() {
        let conn = test_conn();
        log_entries(&conn, 5);
        conn.execute("DELETE FROM audit_logs WHERE seq = 5", [])
            .unwrap();
        let report = verify_chain(&conn).unwrap();
        assert!(!report.valid);
        assert_eq!(report.first_broken_seq, Some(5));
    }

    #[test]
    fn fully_emptied_log_is_detected() {
        let conn = test_conn();
        log_entries(&conn, 3);
        conn.execute("DELETE FROM audit_logs", []).unwrap();
        let report = verify_chain(&conn).unwrap();
        assert!(!report.valid);
        assert_eq!(report.first_broken_seq, Some(1));
    }

    #[test]
    fn legacy_table_migrates_into_valid_chain() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE audit_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                action_type TEXT NOT NULL,
                details TEXT,
                status TEXT NOT NULL,
                error TEXT
            )",
            [],
        )
        .unwrap();
        for i in 0..4 {
            conn.execute(
                "INSERT INTO audit_logs (action_type, details, status, error)
                 VALUES ('legacy_action', ?1, 'success', NULL)",
                params![format!("legacy detail {}", i)],
            )
            .unwrap();
        }

        init_schema(&conn).unwrap();

        let report = verify_chain(&conn).unwrap();
        assert!(report.valid, "{:?}", report);
        assert_eq!(report.entries_checked, 4);
        // Rows preserved with their content and new chain columns populated.
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audit_logs WHERE action_type = 'legacy_action'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 4);
        // And new entries append cleanly to the migrated chain.
        log_action_with_conn(&conn, "post_migration", None, "success", None).unwrap();
        let report = verify_chain(&conn).unwrap();
        assert!(report.valid);
        assert_eq!(report.entries_checked, 5);
    }
}
