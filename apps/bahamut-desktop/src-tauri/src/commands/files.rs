//! Sandboxed file I/O commands: project tree listing, reading, saving with
//! pre-write hash verification + snapshots, and snapshot rollback.
//!
//! Every command revalidates the target path through `validate_path` at the
//! point of use (TOCTOU discipline — validated paths are never cached across
//! user actions), and every save/rollback appends to the hash-chained audit
//! log. Denied attempts are audited too.

use crate::commands::security::validate_path;
use crate::database;
use crate::AppState;
use rusqlite::Connection;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use tauri::State;

/// Directories never shown in the project tree or readable through these
/// commands' listing (reads of explicit paths inside them are still subject
/// to the sandbox only — the filter is a noise/size guard, not a boundary).
const IGNORED_DIRS: [&str; 10] = [
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    "out",
    "__pycache__",
    ".venv",
    "venv",
    ".next",
];

/// Extensions excluded from the tree as known-binary (cheap filter; actual
/// reads additionally reject content containing NUL bytes / invalid UTF-8).
const BINARY_EXTENSIONS: [&str; 32] = [
    "exe", "dll", "so", "dylib", "bin", "o", "obj", "a", "lib", "png", "jpg", "jpeg", "gif", "bmp",
    "ico", "icns", "webp", "pdf", "zip", "gz", "tar", "7z", "rar", "jar", "class", "wasm", "mp3",
    "mp4", "ttf", "woff", "woff2", "db",
];

/// Hard caps so a pathological workspace cannot hang the UI.
const MAX_TREE_ENTRIES: usize = 25_000;
const MAX_TREE_DEPTH: usize = 32;

#[derive(Debug, Serialize)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<FileNode>>,
}

#[derive(Debug, Serialize)]
pub struct FileTreeResponse {
    pub root: String,
    pub nodes: Vec<FileNode>,
    /// True when MAX_TREE_ENTRIES / MAX_TREE_DEPTH stopped the walk early.
    pub truncated: bool,
}

#[derive(Debug, Serialize)]
pub struct ReadFileResponse {
    /// Canonical path — pass this exact value back to `save_project_file`.
    pub path: String,
    pub content: String,
    /// SHA-256 of the bytes read; required by `save_project_file` to detect
    /// concurrent modification.
    pub hash: String,
}

#[derive(Debug, Serialize)]
pub struct SaveFileResponse {
    pub path: String,
    pub new_hash: String,
    /// Pre-change snapshot of the previous content (rollback target).
    pub snapshot_id: i64,
}

#[derive(Debug, Serialize)]
pub struct RollbackResponse {
    pub path: String,
    pub restored_hash: String,
    /// Snapshot of the content that was on disk just before the rollback, so
    /// the rollback itself can be undone.
    pub undo_snapshot_id: Option<i64>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    use std::fmt::Write as _;
    let mut hex = String::with_capacity(64);
    for byte in digest {
        let _ = write!(hex, "{:02x}", byte);
    }
    hex
}

fn is_ignored_dir(name: &str) -> bool {
    IGNORED_DIRS.iter().any(|d| name.eq_ignore_ascii_case(d))
}

fn has_binary_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| {
            BINARY_EXTENSIONS
                .iter()
                .any(|b| ext.eq_ignore_ascii_case(b))
        })
        .unwrap_or(false)
}

struct TreeWalk {
    entries: usize,
    truncated: bool,
    max_file_size: u64,
}

fn walk_dir(dir: &Path, depth: usize, state: &mut TreeWalk) -> Vec<FileNode> {
    if depth > MAX_TREE_DEPTH {
        state.truncated = true;
        return Vec::new();
    }
    let read_dir = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return Vec::new(), // unreadable directory: skip, don't fail the tree
    };

    let mut dirs: Vec<(String, PathBuf)> = Vec::new();
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    for entry in read_dir.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let file_type = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if file_type.is_dir() {
            if !is_ignored_dir(&name) {
                dirs.push((name, path));
            }
        } else if file_type.is_file() {
            if has_binary_extension(&path) {
                continue;
            }
            if let Ok(meta) = entry.metadata() {
                if meta.len() > state.max_file_size {
                    continue;
                }
            }
            files.push((name, path));
        }
        // Symlinks are intentionally omitted from the tree: opening one would
        // be validated (and rejected if it escapes) by read_project_file, but
        // not listing them avoids presenting escape vectors as project files.
    }

    dirs.sort_by_key(|a| a.0.to_lowercase());
    files.sort_by_key(|a| a.0.to_lowercase());

    let mut nodes = Vec::with_capacity(dirs.len() + files.len());
    for (name, path) in dirs {
        if state.entries >= MAX_TREE_ENTRIES {
            state.truncated = true;
            break;
        }
        state.entries += 1;
        let children = walk_dir(&path, depth + 1, state);
        nodes.push(FileNode {
            name,
            path: path.to_string_lossy().to_string(),
            is_dir: true,
            children: Some(children),
        });
    }
    for (name, path) in files {
        if state.entries >= MAX_TREE_ENTRIES {
            state.truncated = true;
            break;
        }
        state.entries += 1;
        nodes.push(FileNode {
            name,
            path: path.to_string_lossy().to_string(),
            is_dir: false,
            children: None,
        });
    }
    nodes
}

/// Builds the filtered tree for `root`. Pure function for testability.
pub fn build_tree(root: &Path, max_file_size: u64) -> Result<FileTreeResponse, String> {
    let canonical_root = root
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize project root: {}", e))?;
    let mut walk = TreeWalk {
        entries: 0,
        truncated: false,
        max_file_size,
    };
    let nodes = walk_dir(&canonical_root, 0, &mut walk);
    Ok(FileTreeResponse {
        root: canonical_root.to_string_lossy().to_string(),
        nodes,
        truncated: walk.truncated,
    })
}

/// Reads a text file inside the sandbox. Rejects out-of-root paths, oversized
/// files, and binary content (NUL bytes or invalid UTF-8).
pub fn read_file_core(
    root: &Path,
    conn: &Connection,
    target: &Path,
) -> Result<ReadFileResponse, String> {
    let validated = validate_path(root, target).inspect_err(|e| {
        let _ = database::log_action_with_conn(
            conn,
            "read_file",
            Some(target.to_string_lossy().to_string()),
            "denied",
            Some(e.clone()),
        );
    })?;

    let max_size = database::get_max_file_size(conn);
    let meta = fs::metadata(&validated).map_err(|e| format!("Failed to stat file: {}", e))?;
    if !meta.is_file() {
        return Err("Not a regular file".to_string());
    }
    if meta.len() > max_size {
        return Err(format!(
            "File exceeds the configured size limit ({} bytes > {} bytes)",
            meta.len(),
            max_size
        ));
    }

    let bytes = fs::read(&validated).map_err(|e| format!("Failed to read file: {}", e))?;
    if bytes.contains(&0) {
        return Err("Binary file: refusing to open in the text editor".to_string());
    }
    let content = String::from_utf8(bytes.clone())
        .map_err(|_| "Binary or non-UTF-8 file: refusing to open in the text editor".to_string())?;

    Ok(ReadFileResponse {
        path: validated.to_string_lossy().to_string(),
        content,
        hash: sha256_hex(&bytes),
    })
}

/// Atomically replaces `target` with `content`: writes a temp file in the
/// same directory, fsyncs it, then renames over the target (MoveFileEx with
/// MOVEFILE_REPLACE_EXISTING on Windows, rename(2) on Unix).
fn atomic_write(target: &Path, content: &[u8]) -> Result<(), String> {
    let dir = target
        .parent()
        .ok_or_else(|| "Target path has no parent directory".to_string())?;
    let tmp_name = format!(
        ".bahamut-tmp-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    );
    let tmp_path = dir.join(tmp_name);

    let write_result = (|| -> std::io::Result<()> {
        let mut tmp = fs::File::create(&tmp_path)?;
        tmp.write_all(content)?;
        tmp.sync_all()?;
        drop(tmp);
        fs::rename(&tmp_path, target)
    })();

    if let Err(e) = write_result {
        let _ = fs::remove_file(&tmp_path);
        return Err(format!("Atomic write failed: {}", e));
    }
    Ok(())
}

/// Saves a file inside the sandbox. The path is revalidated at the point of
/// use; the on-disk content hash must match `expected_hash` (the hash handed
/// out when the file was read) or the save is refused; the previous content
/// is snapshotted before an atomic replace; the action is audit-logged.
pub fn save_file_core(
    root: &Path,
    conn: &Connection,
    target: &Path,
    content: &str,
    expected_hash: &str,
) -> Result<SaveFileResponse, String> {
    let validated = validate_path(root, target).inspect_err(|e| {
        let _ = database::log_action_with_conn(
            conn,
            "save_file",
            Some(target.to_string_lossy().to_string()),
            "denied",
            Some(e.clone()),
        );
    })?;
    let path_str = validated.to_string_lossy().to_string();

    let max_size = database::get_max_file_size(conn);
    if content.len() as u64 > max_size {
        return Err(format!(
            "New content exceeds the configured size limit ({} bytes > {} bytes)",
            content.len(),
            max_size
        ));
    }

    // Pre-write verification: the file must still contain exactly what the
    // editor was given. A mismatch means it changed on disk since it was
    // opened — refuse rather than clobber.
    let current_bytes = match fs::read(&validated) {
        Ok(b) => b,
        Err(e) => {
            let msg = format!("File no longer readable (deleted or moved?): {}", e);
            let _ = database::log_action_with_conn(
                conn,
                "save_file",
                Some(path_str.clone()),
                "conflict",
                Some(msg.clone()),
            );
            return Err(msg);
        }
    };
    let current_hash = sha256_hex(&current_bytes);
    if current_hash != expected_hash {
        let msg =
            "Conflict: file changed on disk since it was opened; refusing stale write".to_string();
        let _ = database::log_action_with_conn(
            conn,
            "save_file",
            Some(path_str.clone()),
            "conflict",
            Some(msg.clone()),
        );
        return Err(msg);
    }
    let current_text = String::from_utf8(current_bytes)
        .map_err(|_| "Refusing to snapshot binary content".to_string())?;

    // Pre-change snapshot, then atomic replace.
    let snapshot_id = database::insert_snapshot(conn, &path_str, &current_text, &current_hash)?;
    atomic_write(&validated, content.as_bytes())?;
    let new_hash = sha256_hex(content.as_bytes());

    database::log_action_with_conn(
        conn,
        "save_file",
        Some(
            serde_json::json!({
                "path": path_str,
                "prev_hash": current_hash,
                "new_hash": new_hash,
                "snapshot_id": snapshot_id,
            })
            .to_string(),
        ),
        "success",
        None,
    )?;

    Ok(SaveFileResponse {
        path: path_str,
        new_hash,
        snapshot_id,
    })
}

/// Restores a snapshot. The stored path is revalidated against the *current*
/// project root at the point of use; the current on-disk content is
/// snapshotted first so the rollback itself is reversible; the restore is an
/// atomic replace and is audit-logged.
pub fn rollback_core(
    root: &Path,
    conn: &Connection,
    snapshot_id: i64,
) -> Result<RollbackResponse, String> {
    let snapshot = database::get_snapshot(conn, snapshot_id)?;
    let target = PathBuf::from(&snapshot.path);
    let validated = validate_path(root, &target).inspect_err(|e| {
        let _ = database::log_action_with_conn(
            conn,
            "rollback_file",
            Some(snapshot.path.clone()),
            "denied",
            Some(e.clone()),
        );
    })?;
    let path_str = validated.to_string_lossy().to_string();

    // Snapshot whatever is on disk right now (if it is still text) so the
    // rollback can be undone.
    let undo_snapshot_id = match fs::read(&validated) {
        Ok(bytes) => {
            let hash = sha256_hex(&bytes);
            match String::from_utf8(bytes) {
                Ok(text) => Some(database::insert_snapshot(conn, &path_str, &text, &hash)?),
                Err(_) => None,
            }
        }
        Err(_) => None,
    };

    atomic_write(&validated, snapshot.content.as_bytes())?;
    let restored_hash = sha256_hex(snapshot.content.as_bytes());
    if restored_hash != snapshot.content_hash {
        return Err("Snapshot integrity check failed after restore".to_string());
    }

    database::log_action_with_conn(
        conn,
        "rollback_file",
        Some(
            serde_json::json!({
                "path": path_str,
                "snapshot_id": snapshot_id,
                "restored_hash": restored_hash,
                "undo_snapshot_id": undo_snapshot_id,
            })
            .to_string(),
        ),
        "success",
        None,
    )?;

    Ok(RollbackResponse {
        path: path_str,
        restored_hash,
        undo_snapshot_id,
    })
}

// ---------------------------------------------------------------------------
// Tauri command wrappers (thin: lock state, delegate to core functions)
// ---------------------------------------------------------------------------

fn with_root_and_conn<T>(
    state: &State<'_, AppState>,
    f: impl FnOnce(&Path, &Connection) -> Result<T, String>,
) -> Result<T, String> {
    let root_guard = state.project_root.lock().map_err(|_| "Mutex error")?;
    let root = root_guard
        .as_ref()
        .ok_or_else(|| "No project folder opened".to_string())?;
    let conn_guard = state.db_conn.lock().map_err(|_| "Mutex error")?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database connection not initialized".to_string())?;
    f(root, conn)
}

#[tauri::command]
pub fn list_project_files(state: State<'_, AppState>) -> Result<FileTreeResponse, String> {
    with_root_and_conn(&state, |root, conn| {
        build_tree(root, database::get_max_file_size(conn))
    })
}

#[tauri::command]
pub fn read_project_file(
    state: State<'_, AppState>,
    path: String,
) -> Result<ReadFileResponse, String> {
    with_root_and_conn(&state, |root, conn| {
        read_file_core(root, conn, &PathBuf::from(path))
    })
}

#[tauri::command]
pub fn save_project_file(
    state: State<'_, AppState>,
    path: String,
    content: String,
    expected_hash: String,
) -> Result<SaveFileResponse, String> {
    with_root_and_conn(&state, |root, conn| {
        save_file_core(root, conn, &PathBuf::from(path), &content, &expected_hash)
    })
}

#[tauri::command]
pub fn rollback_file_snapshot(
    state: State<'_, AppState>,
    snapshot_id: i64,
) -> Result<RollbackResponse, String> {
    with_root_and_conn(&state, |root, conn| rollback_core(root, conn, snapshot_id))
}

#[tauri::command]
pub fn list_file_snapshots(
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<database::SnapshotMeta>, String> {
    with_root_and_conn(&state, |root, conn| {
        let validated = validate_path(root, &PathBuf::from(path))?;
        database::list_snapshots_for_path(conn, &validated.to_string_lossy())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{init_schema, verify_chain};

    struct TestEnv {
        base: PathBuf,
        root: PathBuf,
        conn: Connection,
    }

    impl TestEnv {
        fn new(tag: &str) -> Self {
            let base =
                std::env::temp_dir().join(format!("bahamut_files_{}_{}", tag, std::process::id()));
            let _ = fs::remove_dir_all(&base);
            let root = base.join("project");
            fs::create_dir_all(root.join("src")).unwrap();
            fs::write(root.join("README.md"), "hello bahamut\n").unwrap();
            fs::write(root.join("src").join("main.rs"), "fn main() {}\n").unwrap();
            fs::write(base.join("outside.txt"), "outside secret").unwrap();

            let conn = Connection::open_in_memory().unwrap();
            init_schema(&conn).unwrap();
            TestEnv { base, root, conn }
        }

        fn read_readme(&self) -> ReadFileResponse {
            read_file_core(&self.root, &self.conn, &self.root.join("README.md")).unwrap()
        }

        fn audit_rows(&self, action: &str, status: &str) -> i64 {
            self.conn
                .query_row(
                    "SELECT COUNT(*) FROM audit_logs WHERE action_type = ?1 AND status = ?2",
                    rusqlite::params![action, status],
                    |row| row.get(0),
                )
                .unwrap()
        }
    }

    impl Drop for TestEnv {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.base);
        }
    }

    #[test]
    fn tree_excludes_ignored_dirs_binary_and_oversized_files() {
        let t = TestEnv::new("tree");
        fs::create_dir_all(t.root.join("node_modules").join("pkg")).unwrap();
        fs::write(t.root.join("node_modules").join("pkg").join("i.js"), "x").unwrap();
        fs::create_dir_all(t.root.join(".git")).unwrap();
        fs::write(t.root.join(".git").join("HEAD"), "ref").unwrap();
        fs::write(t.root.join("logo.png"), [0u8; 8]).unwrap();
        fs::write(t.root.join("big.txt"), vec![b'a'; 4096]).unwrap();

        let tree = build_tree(&t.root, 1024).unwrap(); // 1 KiB limit
        let names: Vec<&str> = tree.nodes.iter().map(|n| n.name.as_str()).collect();
        assert!(names.contains(&"src"));
        assert!(names.contains(&"README.md"));
        assert!(!names.contains(&"node_modules"));
        assert!(!names.contains(&".git"));
        assert!(!names.contains(&"logo.png"), "binary extension filtered");
        assert!(!names.contains(&"big.txt"), "oversized file filtered");
        assert!(!tree.truncated);
    }

    #[test]
    fn read_returns_content_and_hash() {
        let t = TestEnv::new("read");
        let resp = t.read_readme();
        assert_eq!(resp.content, "hello bahamut\n");
        assert_eq!(resp.hash, sha256_hex(b"hello bahamut\n"));
    }

    #[test]
    fn read_rejects_path_outside_root_and_audits_denial() {
        let t = TestEnv::new("read_outside");
        let err = read_file_core(&t.root, &t.conn, &t.base.join("outside.txt")).unwrap_err();
        assert!(err.contains("outside"), "{}", err);
        assert_eq!(t.audit_rows("read_file", "denied"), 1);

        let err =
            read_file_core(&t.root, &t.conn, &t.root.join("..").join("outside.txt")).unwrap_err();
        assert!(!err.is_empty());
        assert_eq!(t.audit_rows("read_file", "denied"), 2);
    }

    #[test]
    fn read_rejects_binary_and_oversized_content() {
        let t = TestEnv::new("read_binary");
        fs::write(t.root.join("blob.dat"), [1u8, 0u8, 2u8]).unwrap();
        let err = read_file_core(&t.root, &t.conn, &t.root.join("blob.dat")).unwrap_err();
        assert!(err.contains("Binary"), "{}", err);

        let max = database::get_max_file_size(&t.conn);
        fs::write(t.root.join("huge.txt"), vec![b'x'; (max + 1) as usize]).unwrap();
        let err = read_file_core(&t.root, &t.conn, &t.root.join("huge.txt")).unwrap_err();
        assert!(err.contains("size limit"), "{}", err);
    }

    #[test]
    fn save_writes_atomically_snapshots_and_audits() {
        let t = TestEnv::new("save");
        let opened = t.read_readme();
        let resp = save_file_core(
            &t.root,
            &t.conn,
            &t.root.join("README.md"),
            "updated content\n",
            &opened.hash,
        )
        .unwrap();

        assert_eq!(
            fs::read_to_string(t.root.join("README.md")).unwrap(),
            "updated content\n"
        );
        assert_eq!(resp.new_hash, sha256_hex(b"updated content\n"));

        // Pre-change snapshot holds the original content.
        let snap = database::get_snapshot(&t.conn, resp.snapshot_id).unwrap();
        assert_eq!(snap.content, "hello bahamut\n");
        assert_eq!(snap.content_hash, opened.hash);

        // No temp files left behind.
        let leftovers: Vec<_> = fs::read_dir(&t.root)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().contains("bahamut-tmp"))
            .collect();
        assert!(leftovers.is_empty());

        // Audited and chain still valid.
        assert_eq!(t.audit_rows("save_file", "success"), 1);
        let report = verify_chain(&t.conn).unwrap();
        assert!(report.valid, "{:?}", report);
    }

    #[test]
    fn save_rejects_stale_hash_and_leaves_file_untouched() {
        let t = TestEnv::new("save_stale");
        let opened = t.read_readme();
        // File changes on disk after it was opened.
        fs::write(t.root.join("README.md"), "changed externally\n").unwrap();

        let err = save_file_core(
            &t.root,
            &t.conn,
            &t.root.join("README.md"),
            "my edit\n",
            &opened.hash,
        )
        .unwrap_err();
        assert!(err.contains("Conflict"), "{}", err);
        assert_eq!(
            fs::read_to_string(t.root.join("README.md")).unwrap(),
            "changed externally\n"
        );
        assert_eq!(t.audit_rows("save_file", "conflict"), 1);
    }

    #[test]
    fn save_rejects_outside_paths_and_traversal() {
        let t = TestEnv::new("save_outside");
        let err = save_file_core(
            &t.root,
            &t.conn,
            &t.base.join("outside.txt"),
            "evil",
            &sha256_hex(b"outside secret"),
        )
        .unwrap_err();
        assert!(err.contains("outside"), "{}", err);
        assert_eq!(
            fs::read_to_string(t.base.join("outside.txt")).unwrap(),
            "outside secret"
        );

        let err = save_file_core(
            &t.root,
            &t.conn,
            &t.root.join("src").join("..").join("..").join("outside.txt"),
            "evil",
            &sha256_hex(b"outside secret"),
        )
        .unwrap_err();
        assert!(!err.is_empty());
        assert_eq!(t.audit_rows("save_file", "denied"), 2);
    }

    #[test]
    fn save_rejects_oversized_content() {
        let t = TestEnv::new("save_big");
        let opened = t.read_readme();
        let max = database::get_max_file_size(&t.conn);
        let big = "x".repeat((max + 1) as usize);
        let err = save_file_core(
            &t.root,
            &t.conn,
            &t.root.join("README.md"),
            &big,
            &opened.hash,
        )
        .unwrap_err();
        assert!(err.contains("size limit"), "{}", err);
    }

    #[test]
    fn rollback_restores_previous_content_with_undo_snapshot_and_audit() {
        let t = TestEnv::new("rollback");
        let opened = t.read_readme();
        let saved = save_file_core(
            &t.root,
            &t.conn,
            &t.root.join("README.md"),
            "version two\n",
            &opened.hash,
        )
        .unwrap();

        let rb = rollback_core(&t.root, &t.conn, saved.snapshot_id).unwrap();
        assert_eq!(
            fs::read_to_string(t.root.join("README.md")).unwrap(),
            "hello bahamut\n"
        );
        assert_eq!(rb.restored_hash, opened.hash);

        // The undo snapshot captures "version two" so the rollback is reversible.
        let undo = database::get_snapshot(&t.conn, rb.undo_snapshot_id.unwrap()).unwrap();
        assert_eq!(undo.content, "version two\n");

        assert_eq!(t.audit_rows("rollback_file", "success"), 1);
        let report = verify_chain(&t.conn).unwrap();
        assert!(report.valid, "{:?}", report);
    }

    #[test]
    fn rollback_revalidates_path_against_current_root() {
        let t = TestEnv::new("rollback_revalidate");
        let opened = t.read_readme();
        let saved = save_file_core(
            &t.root,
            &t.conn,
            &t.root.join("README.md"),
            "version two\n",
            &opened.hash,
        )
        .unwrap();

        // Same DB, but the active project root is now a different directory:
        // the stored snapshot path no longer falls inside it.
        let other_root = t.base.join("other_project");
        fs::create_dir_all(&other_root).unwrap();
        let err = rollback_core(&other_root, &t.conn, saved.snapshot_id).unwrap_err();
        assert!(err.contains("outside"), "{}", err);
        assert_eq!(t.audit_rows("rollback_file", "denied"), 1);
        // File untouched.
        assert_eq!(
            fs::read_to_string(t.root.join("README.md")).unwrap(),
            "version two\n"
        );
    }

    #[test]
    fn snapshot_listing_is_scoped_to_validated_path() {
        let t = TestEnv::new("snapshot_list");
        let opened = t.read_readme();
        save_file_core(
            &t.root,
            &t.conn,
            &t.root.join("README.md"),
            "v2\n",
            &opened.hash,
        )
        .unwrap();

        let validated = validate_path(&t.root, &t.root.join("README.md")).unwrap();
        let snaps =
            database::list_snapshots_for_path(&t.conn, &validated.to_string_lossy()).unwrap();
        assert_eq!(snaps.len(), 1);
        assert_eq!(snaps[0].content_hash, opened.hash);
    }
}
