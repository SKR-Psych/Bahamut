//! Secure create / rename / delete operations for files and folders.
//!
//! Every operation revalidates its path(s) through `validate_path` at the
//! point of use (inheriting traversal, symlink-escape, NTFS ADS, and reserved
//! device-name protections), refuses overwrites, and appends an audit entry
//! for successes, denials, and failures. Deletes are recoverable: the target
//! is moved into a trash folder under the application data directory, and
//! text files are additionally snapshotted to SQLite first.

use crate::commands::files::{sha256_hex, with_root_and_conn};
use crate::commands::security::validate_path;
use crate::database;
use crate::AppState;
use rusqlite::Connection;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::State;

#[derive(Debug, Serialize)]
pub struct FileOpResponse {
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct RenameResponse {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub path: String,
    /// Where the deleted entry was moved (recoverable until manually purged).
    pub trash_path: String,
    /// SQLite snapshot of the file content, when the target was a text file
    /// within the size limit.
    pub snapshot_id: Option<i64>,
}

fn audit_denied(conn: &Connection, action: &str, detail: String, err: &str) {
    let _ = database::log_action_with_conn(conn, action, Some(detail), "denied", Some(err.into()));
}

fn audit_failure(conn: &Connection, action: &str, detail: String, err: &str) {
    let _ = database::log_action_with_conn(conn, action, Some(detail), "failure", Some(err.into()));
}

fn audit_success(conn: &Connection, action: &str, detail: String) -> Result<(), String> {
    database::log_action_with_conn(conn, action, Some(detail), "success", None)
}

/// Creates an empty file. Fails if the file already exists (no overwrite).
pub fn create_file_core(
    root: &Path,
    conn: &Connection,
    target: &Path,
) -> Result<FileOpResponse, String> {
    let requested = target.to_string_lossy().to_string();
    let validated = validate_path(root, target).inspect_err(|e| {
        audit_denied(conn, "create_file", requested.clone(), e);
    })?;
    let path_str = validated.to_string_lossy().to_string();

    if let Some(parent) = validated.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            let msg = format!("Failed to create parent directories: {}", e);
            audit_failure(conn, "create_file", path_str.clone(), &msg);
            msg
        })?;
    }

    // create_new fails if the file exists — refuses silent overwrite and is
    // atomic (no TOCTOU window between an exists-check and the create).
    fs::File::create_new(&validated).map_err(|e| {
        let msg = if e.kind() == std::io::ErrorKind::AlreadyExists {
            "A file with this name already exists".to_string()
        } else {
            format!("Failed to create file: {}", e)
        };
        audit_failure(conn, "create_file", path_str.clone(), &msg);
        msg
    })?;

    audit_success(conn, "create_file", path_str.clone())?;
    Ok(FileOpResponse { path: path_str })
}

/// Creates a folder (with intermediate directories). Fails if it exists.
pub fn create_folder_core(
    root: &Path,
    conn: &Connection,
    target: &Path,
) -> Result<FileOpResponse, String> {
    let requested = target.to_string_lossy().to_string();
    let validated = validate_path(root, target).inspect_err(|e| {
        audit_denied(conn, "create_folder", requested.clone(), e);
    })?;
    let path_str = validated.to_string_lossy().to_string();

    if validated.exists() {
        let msg = "An entry with this name already exists".to_string();
        audit_failure(conn, "create_folder", path_str.clone(), &msg);
        return Err(msg);
    }
    fs::create_dir_all(&validated).map_err(|e| {
        let msg = format!("Failed to create folder: {}", e);
        audit_failure(conn, "create_folder", path_str.clone(), &msg);
        msg
    })?;

    audit_success(conn, "create_folder", path_str.clone())?;
    Ok(FileOpResponse { path: path_str })
}

/// Renames/moves a file or folder within the sandbox. The destination must
/// not exist (no overwrite), and both endpoints are validated.
pub fn rename_core(
    root: &Path,
    conn: &Connection,
    from: &Path,
    to: &Path,
) -> Result<RenameResponse, String> {
    let requested = format!("{} -> {}", from.to_string_lossy(), to.to_string_lossy());
    let validated_from = validate_path(root, from).inspect_err(|e| {
        audit_denied(conn, "rename_path", requested.clone(), e);
    })?;
    let validated_to = validate_path(root, to).inspect_err(|e| {
        audit_denied(conn, "rename_path", requested.clone(), e);
    })?;
    let detail = format!(
        "{} -> {}",
        validated_from.to_string_lossy(),
        validated_to.to_string_lossy()
    );

    if !validated_from.exists() {
        let msg = "Source no longer exists".to_string();
        audit_failure(conn, "rename_path", detail.clone(), &msg);
        return Err(msg);
    }
    let canonical_root = root
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize project root: {}", e))?;
    if validated_from == canonical_root {
        let msg = "Refusing to rename the project root".to_string();
        audit_denied(conn, "rename_path", detail.clone(), &msg);
        return Err(msg);
    }
    if validated_to.exists() {
        let msg = "Destination already exists; refusing to overwrite".to_string();
        audit_failure(conn, "rename_path", detail.clone(), &msg);
        return Err(msg);
    }

    fs::rename(&validated_from, &validated_to).map_err(|e| {
        let msg = format!("Rename failed: {}", e);
        audit_failure(conn, "rename_path", detail.clone(), &msg);
        msg
    })?;

    audit_success(conn, "rename_path", detail)?;
    Ok(RenameResponse {
        from: validated_from.to_string_lossy().to_string(),
        to: validated_to.to_string_lossy().to_string(),
    })
}

/// Recoverable delete: moves the target into `trash_dir` (timestamped name)
/// instead of unlinking it. Text files within the size limit are also
/// snapshotted to SQLite (operation = "pre-delete") for in-app recovery.
pub fn delete_core(
    root: &Path,
    conn: &Connection,
    trash_dir: &Path,
    target: &Path,
) -> Result<DeleteResponse, String> {
    let requested = target.to_string_lossy().to_string();
    let validated = validate_path(root, target).inspect_err(|e| {
        audit_denied(conn, "delete_path", requested.clone(), e);
    })?;
    let path_str = validated.to_string_lossy().to_string();

    if !validated.exists() {
        let msg = "Target no longer exists".to_string();
        audit_failure(conn, "delete_path", path_str.clone(), &msg);
        return Err(msg);
    }
    let canonical_root = root
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize project root: {}", e))?;
    if validated == canonical_root {
        let msg = "Refusing to delete the project root".to_string();
        audit_denied(conn, "delete_path", path_str.clone(), &msg);
        return Err(msg);
    }

    // Snapshot text-file content for in-app recovery before anything moves.
    let mut snapshot_id = None;
    if validated.is_file() {
        let max_size = database::get_max_file_size(conn);
        if let Ok(meta) = fs::metadata(&validated) {
            if meta.len() <= max_size {
                if let Ok(bytes) = fs::read(&validated) {
                    let hash = sha256_hex(&bytes);
                    if let Ok(text) = String::from_utf8(bytes) {
                        snapshot_id = Some(database::insert_snapshot(
                            conn,
                            &path_str,
                            &text,
                            &hash,
                            "pre-delete",
                        )?);
                    }
                }
            }
        }
    }

    fs::create_dir_all(trash_dir).map_err(|e| {
        let msg = format!("Failed to prepare trash folder: {}", e);
        audit_failure(conn, "delete_path", path_str.clone(), &msg);
        msg
    })?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let file_name = validated
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "entry".to_string());
    let trash_path = trash_dir.join(format!("{}-{}", stamp, file_name));

    fs::rename(&validated, &trash_path).map_err(|e| {
        let msg = format!(
            "Failed to move to trash (cross-volume moves are not supported): {}",
            e
        );
        audit_failure(conn, "delete_path", path_str.clone(), &msg);
        msg
    })?;

    let trash_str = trash_path.to_string_lossy().to_string();
    audit_success(
        conn,
        "delete_path",
        serde_json::json!({
            "path": path_str,
            "trash_path": trash_str,
            "snapshot_id": snapshot_id,
        })
        .to_string(),
    )?;

    Ok(DeleteResponse {
        path: path_str,
        trash_path: trash_str,
        snapshot_id,
    })
}

// ---------------------------------------------------------------------------
// Tauri command wrappers
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn create_project_file(
    state: State<'_, AppState>,
    path: String,
) -> Result<FileOpResponse, String> {
    with_root_and_conn(&state, |root, conn| {
        create_file_core(root, conn, &PathBuf::from(path))
    })
}

#[tauri::command]
pub fn create_project_folder(
    state: State<'_, AppState>,
    path: String,
) -> Result<FileOpResponse, String> {
    with_root_and_conn(&state, |root, conn| {
        create_folder_core(root, conn, &PathBuf::from(path))
    })
}

#[tauri::command]
pub fn rename_project_path(
    state: State<'_, AppState>,
    from: String,
    to: String,
) -> Result<RenameResponse, String> {
    with_root_and_conn(&state, |root, conn| {
        rename_core(root, conn, &PathBuf::from(from), &PathBuf::from(to))
    })
}

#[tauri::command]
pub fn delete_project_path(
    state: State<'_, AppState>,
    path: String,
) -> Result<DeleteResponse, String> {
    let trash_dir = state.app_data_dir.join("trash");
    with_root_and_conn(&state, |root, conn| {
        delete_core(root, conn, &trash_dir, &PathBuf::from(path))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{init_schema, verify_chain};

    struct TestEnv {
        base: PathBuf,
        root: PathBuf,
        trash: PathBuf,
        conn: Connection,
    }

    impl TestEnv {
        fn new(tag: &str) -> Self {
            let base = std::env::temp_dir().join(format!(
                "bahamut_fileops_{}_{}",
                tag,
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&base);
            let root = base.join("project");
            let trash = base.join("trash");
            fs::create_dir_all(root.join("src")).unwrap();
            fs::write(root.join("notes.txt"), "keep me\n").unwrap();
            fs::write(base.join("outside.txt"), "secret").unwrap();

            let conn = Connection::open_in_memory().unwrap();
            init_schema(&conn).unwrap();
            TestEnv {
                base,
                root,
                trash,
                conn,
            }
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
    fn creates_file_and_folder_inside_sandbox_with_audit() {
        let t = TestEnv::new("create");
        let f = create_file_core(&t.root, &t.conn, &t.root.join("src").join("new.rs")).unwrap();
        assert!(PathBuf::from(&f.path).exists());

        let d = create_folder_core(&t.root, &t.conn, &t.root.join("docs")).unwrap();
        assert!(PathBuf::from(&d.path).is_dir());

        assert_eq!(t.audit_rows("create_file", "success"), 1);
        assert_eq!(t.audit_rows("create_folder", "success"), 1);
        assert!(verify_chain(&t.conn).unwrap().valid);
    }

    #[test]
    fn create_refuses_overwrite() {
        let t = TestEnv::new("create_overwrite");
        let err = create_file_core(&t.root, &t.conn, &t.root.join("notes.txt")).unwrap_err();
        assert!(err.contains("already exists"), "{}", err);
        assert_eq!(
            fs::read_to_string(t.root.join("notes.txt")).unwrap(),
            "keep me\n"
        );
        assert_eq!(t.audit_rows("create_file", "failure"), 1);

        let err = create_folder_core(&t.root, &t.conn, &t.root.join("src")).unwrap_err();
        assert!(err.contains("already exists"), "{}", err);
    }

    #[test]
    fn create_rejects_outside_traversal_ads_and_device_paths() {
        let t = TestEnv::new("create_reject");
        assert!(create_file_core(&t.root, &t.conn, &t.base.join("evil.txt")).is_err());
        assert!(create_file_core(&t.root, &t.conn, &t.root.join("..").join("evil.txt")).is_err());
        #[cfg(windows)]
        {
            assert!(create_file_core(&t.root, &t.conn, &t.root.join("file.txt:ads")).is_err());
            assert!(create_file_core(&t.root, &t.conn, &t.root.join("NUL")).is_err());
            assert!(create_folder_core(&t.root, &t.conn, &t.root.join("COM1")).is_err());
        }
        assert!(t.audit_rows("create_file", "denied") >= 2);
    }

    #[test]
    fn renames_inside_sandbox_and_refuses_overwrite() {
        let t = TestEnv::new("rename");
        let r = rename_core(
            &t.root,
            &t.conn,
            &t.root.join("notes.txt"),
            &t.root.join("renamed.txt"),
        )
        .unwrap();
        assert!(!t.root.join("notes.txt").exists());
        assert_eq!(
            fs::read_to_string(PathBuf::from(&r.to)).unwrap(),
            "keep me\n"
        );
        assert_eq!(t.audit_rows("rename_path", "success"), 1);

        // Refuses to clobber an existing destination.
        fs::write(t.root.join("other.txt"), "other").unwrap();
        let err = rename_core(
            &t.root,
            &t.conn,
            &t.root.join("renamed.txt"),
            &t.root.join("other.txt"),
        )
        .unwrap_err();
        assert!(err.contains("refusing to overwrite"), "{}", err);
        assert_eq!(
            fs::read_to_string(t.root.join("other.txt")).unwrap(),
            "other"
        );
        assert_eq!(t.audit_rows("rename_path", "failure"), 1);
    }

    #[test]
    fn rename_rejects_endpoints_outside_sandbox() {
        let t = TestEnv::new("rename_outside");
        // Source outside.
        assert!(rename_core(
            &t.root,
            &t.conn,
            &t.base.join("outside.txt"),
            &t.root.join("stolen.txt"),
        )
        .is_err());
        assert!(t.base.join("outside.txt").exists());
        // Destination outside.
        assert!(rename_core(
            &t.root,
            &t.conn,
            &t.root.join("notes.txt"),
            &t.base.join("leaked.txt"),
        )
        .is_err());
        assert!(t.root.join("notes.txt").exists());
        assert!(!t.base.join("leaked.txt").exists());
        assert_eq!(t.audit_rows("rename_path", "denied"), 2);
    }

    #[test]
    fn delete_moves_to_trash_and_snapshots_text_files() {
        let t = TestEnv::new("delete");
        let resp = delete_core(&t.root, &t.conn, &t.trash, &t.root.join("notes.txt")).unwrap();
        assert!(!t.root.join("notes.txt").exists());
        let trashed = PathBuf::from(&resp.trash_path);
        assert!(trashed.exists());
        assert_eq!(fs::read_to_string(&trashed).unwrap(), "keep me\n");

        // SQLite snapshot captured for in-app recovery.
        let snap_id = resp.snapshot_id.expect("text file should be snapshotted");
        let snap = database::get_snapshot(&t.conn, snap_id).unwrap();
        assert_eq!(snap.content, "keep me\n");
        assert_eq!(snap.operation, "pre-delete");

        assert_eq!(t.audit_rows("delete_path", "success"), 1);
        assert!(verify_chain(&t.conn).unwrap().valid);
    }

    #[test]
    fn delete_folder_is_recoverable_from_trash() {
        let t = TestEnv::new("delete_folder");
        fs::write(t.root.join("src").join("main.rs"), "fn main() {}\n").unwrap();
        let resp = delete_core(&t.root, &t.conn, &t.trash, &t.root.join("src")).unwrap();
        assert!(!t.root.join("src").exists());
        let trashed = PathBuf::from(&resp.trash_path);
        assert_eq!(
            fs::read_to_string(trashed.join("main.rs")).unwrap(),
            "fn main() {}\n"
        );
    }

    #[test]
    fn delete_rejects_outside_paths_and_project_root() {
        let t = TestEnv::new("delete_reject");
        assert!(delete_core(&t.root, &t.conn, &t.trash, &t.base.join("outside.txt")).is_err());
        assert!(t.base.join("outside.txt").exists());

        let err = delete_core(&t.root, &t.conn, &t.trash, &t.root).unwrap_err();
        assert!(err.contains("project root"), "{}", err);
        assert!(t.root.exists());
        assert_eq!(t.audit_rows("delete_path", "denied"), 2);
    }

    #[cfg(unix)]
    #[test]
    fn ops_reject_symlink_escape() {
        let t = TestEnv::new("symlink");
        let link = t.root.join("link.txt");
        std::os::unix::fs::symlink(t.base.join("outside.txt"), &link).unwrap();
        assert!(delete_core(&t.root, &t.conn, &t.trash, &link).is_err());
        assert!(rename_core(&t.root, &t.conn, &link, &t.root.join("renamed.txt")).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn ops_reject_symlink_escape() {
        let t = TestEnv::new("symlink");
        let link = t.root.join("link.txt");
        match std::os::windows::fs::symlink_file(t.base.join("outside.txt"), &link) {
            Ok(()) => {}
            Err(e) if e.raw_os_error() == Some(1314) => {
                eprintln!("skipping symlink assertion: privilege not held");
                return;
            }
            Err(e) => panic!("symlink_file failed: {}", e),
        }
        assert!(delete_core(&t.root, &t.conn, &t.trash, &link).is_err());
        assert!(rename_core(&t.root, &t.conn, &link, &t.root.join("renamed.txt")).is_err());
        // The out-of-root target is untouched.
        assert_eq!(
            fs::read_to_string(t.base.join("outside.txt")).unwrap(),
            "secret"
        );
    }
}
