use std::path::{Path, PathBuf};
use tauri::{AppHandle, State, Wry};
use crate::AppState;

/// Validates that `target_path` resolves to a location strictly inside `project_root`.
/// Handles symlinks and path traversals by resolving canonical paths.
pub fn validate_path(project_root: &Path, target_path: &Path) -> Result<PathBuf, String> {
    // Canonicalize project root.
    let canonical_root = project_root.canonicalize()
        .map_err(|e| format!("Failed to canonicalize project root: {}", e))?;

    // If target path exists, we can canonicalize it directly.
    let canonical_target = if target_path.exists() {
        target_path.canonicalize()
            .map_err(|e| format!("Failed to canonicalize target path: {}", e))?
    } else {
        // If it doesn't exist, find the nearest existing parent directory.
        let mut parent = target_path.parent();
        let mut existing_parent = None;
        while let Some(p) = parent {
            if p.exists() {
                existing_parent = Some(p);
                break;
            }
            parent = p.parent();
        }

        match existing_parent {
            Some(p) => {
                let canonical_parent = p.canonicalize()
                    .map_err(|e| format!("Failed to canonicalize parent path: {}", e))?;
                
                // Re-append the non-existing components to verify they don't contain parent directory traversals
                let relative = target_path.strip_prefix(p)
                    .map_err(|_| "Failed to resolve path components".to_string())?;
                
                if relative.components().any(|c| c == std::path::Component::ParentDir) {
                    return Err("Path traversal (..) sequence detected in non-existent path".to_string());
                }
                
                canonical_parent.join(relative)
            }
            None => {
                return Err("Invalid path: no existing parent directory found".to_string());
            }
        }
    };

    // Check if the target is within the root.
    if canonical_target.starts_with(&canonical_root) {
        Ok(canonical_target)
    } else {
        Err("Access denied: path is outside the project workspace".to_string())
    }
}

#[tauri::command]
pub fn set_project_root(state: State<'_, AppState>, path: String) -> Result<String, String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() || !path_buf.is_dir() {
        return Err("Selected path does not exist or is not a directory".to_string());
    }

    // Canonicalize it to store it safely
    let canonical = path_buf.canonicalize()
        .map_err(|e| format!("Failed to canonicalize path: {}", e))?;

    let mut root_guard = state.project_root.lock().map_err(|_| "Mutex error")?;
    *root_guard = Some(canonical.clone());

    // Log the selection in the audit log
    if let Err(e) = crate::database::log_action(
        &state,
        "set_project_root",
        Some(canonical.to_string_lossy().to_string()),
        "success",
        None
    ) {
        println!("Failed to write to audit log: {}", e);
    }

    Ok(canonical.to_string_lossy().to_string())
}

#[tauri::command]
pub fn check_file_in_sandbox(state: State<'_, AppState>, path: String) -> Result<bool, String> {
    let root_guard = state.project_root.lock().map_err(|_| "Mutex error")?;
    let root = match &*root_guard {
        Some(r) => r,
        None => return Err("No project folder opened".to_string()),
    };

    let target = PathBuf::from(&path);
    match validate_path(root, &target) {
        Ok(_) => Ok(true),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_validate_path_sandbox() {
        let system_temp = std::env::temp_dir();
        let unique_test_dir = system_temp.join("bahamut_test_sandbox");
        let _ = fs::remove_dir_all(&unique_test_dir); // clean up old runs
        fs::create_dir_all(&unique_test_dir).unwrap();

        let root = unique_test_dir.join("project");
        let src = root.join("src");
        fs::create_dir_all(&src).unwrap();

        let config_file = root.join("config.json");
        fs::write(&config_file, "{}").unwrap();

        let child_file = src.join("main.rs");
        fs::write(&child_file, "fn main() {}").unwrap();

        // 1. Success cases
        assert!(validate_path(&root, &config_file).is_ok());
        assert!(validate_path(&root, &child_file).is_ok());
        
        // 2. Traversal outside sandbox
        let outside = unique_test_dir.join("outside.txt");
        fs::write(&outside, "secret").unwrap();
        assert!(validate_path(&root, &outside).is_err());

        // 3. Parent directory dots traversal attack
        let dots_traversal = src.join("../../outside.txt");
        assert!(validate_path(&root, &dots_traversal).is_err());

        // Clean up
        let _ = fs::remove_dir_all(&unique_test_dir);
    }
}
