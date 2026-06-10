use crate::AppState;
use std::path::{Path, PathBuf};
use tauri::State;

/// Validates that `target_path` resolves to a location strictly inside `project_root`.
/// Handles symlinks and path traversals by resolving canonical paths.
///
/// # Guarantee and limits (TOCTOU)
///
/// The verdict is correct at time-of-check only: the returned path is *not* a
/// capability. The filesystem can change between validation and use (e.g. a
/// validated non-existent leaf can be created as a symlink pointing outside
/// the root). Callers performing file I/O must therefore re-validate
/// immediately before the open/write, and must never cache a validated path
/// across user actions. When file I/O commands are added they should open
/// first and verify the opened handle (or use `O_NOFOLLOW`-style open flags /
/// `create_new`) rather than trusting an earlier validation.
pub fn validate_path(project_root: &Path, target_path: &Path) -> Result<PathBuf, String> {
    // Windows-only lexical guards that canonicalization does not catch:
    // - NTFS alternate data stream syntax (`file.txt:stream`, `file.txt::$DATA`)
    //   can smuggle data past extension and content checks, and is never a
    //   legal Windows file name (':' is reserved outside the drive prefix).
    // - Reserved DOS device names (`NUL`, `CON`, `COM1`, …) inside the root
    //   are redirected to devices by the Win32 namespace, so a "validated"
    //   in-root path like `project\NUL` would actually open a device.
    #[cfg(windows)]
    {
        use std::path::Component;
        const RESERVED_DEVICE_NAMES: [&str; 22] = [
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
            "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
        ];
        for component in target_path.components() {
            if let Component::Normal(os) = component {
                let name = os.to_string_lossy();
                if name.contains(':') {
                    return Err(
                        "Access denied: NTFS alternate data stream syntax is not allowed"
                            .to_string(),
                    );
                }
                // Device names are reserved with any extension (`NUL.txt`).
                let stem = name.split('.').next().unwrap_or("").trim_end();
                if RESERVED_DEVICE_NAMES
                    .iter()
                    .any(|r| stem.eq_ignore_ascii_case(r))
                {
                    return Err("Access denied: reserved Windows device name in path".to_string());
                }
            }
        }
    }

    // Canonicalize project root.
    let canonical_root = project_root
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize project root: {}", e))?;

    // If target path exists, we can canonicalize it directly.
    let canonical_target = if target_path.exists() {
        target_path
            .canonicalize()
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
                let canonical_parent = p
                    .canonicalize()
                    .map_err(|e| format!("Failed to canonicalize parent path: {}", e))?;

                // Re-append the non-existing components to verify they don't contain parent directory traversals
                let relative = target_path
                    .strip_prefix(p)
                    .map_err(|_| "Failed to resolve path components".to_string())?;

                if relative
                    .components()
                    .any(|c| c == std::path::Component::ParentDir)
                {
                    return Err(
                        "Path traversal (..) sequence detected in non-existent path".to_string()
                    );
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
    let canonical = path_buf
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize path: {}", e))?;

    let mut root_guard = state.project_root.lock().map_err(|_| "Mutex error")?;
    *root_guard = Some(canonical.clone());

    // Log the selection in the audit log
    if let Err(e) = crate::database::log_action(
        &state,
        "set_project_root",
        Some(canonical.to_string_lossy().to_string()),
        "success",
        None,
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

    /// Per-test unique sandbox layout:
    /// base/
    ///   project/          <- the sandbox root
    ///     config.json
    ///     src/main.rs
    ///   outside.txt       <- outside the sandbox
    ///   outside_dir/secret.txt
    struct TestDirs {
        base: PathBuf,
        root: PathBuf,
    }

    impl TestDirs {
        fn new(tag: &str) -> Self {
            let base = std::env::temp_dir().join(format!(
                "bahamut_sandbox_{}_{}",
                tag,
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&base);
            let root = base.join("project");
            fs::create_dir_all(root.join("src")).unwrap();
            fs::write(root.join("config.json"), "{}").unwrap();
            fs::write(root.join("src").join("main.rs"), "fn main() {}").unwrap();
            fs::write(base.join("outside.txt"), "secret").unwrap();
            fs::create_dir_all(base.join("outside_dir")).unwrap();
            fs::write(base.join("outside_dir").join("secret.txt"), "secret").unwrap();
            TestDirs { base, root }
        }
    }

    impl Drop for TestDirs {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.base);
        }
    }

    #[cfg(unix)]
    fn make_symlink_file(original: &Path, link: &Path) -> Option<()> {
        std::os::unix::fs::symlink(original, link).unwrap();
        Some(())
    }

    #[cfg(unix)]
    fn make_symlink_dir(original: &Path, link: &Path) -> Option<()> {
        std::os::unix::fs::symlink(original, link).unwrap();
        Some(())
    }

    // On Windows creating symlinks needs SeCreateSymbolicLinkPrivilege (admin
    // or Developer Mode); skip symlink tests gracefully when it is not held.
    #[cfg(windows)]
    fn make_symlink_file(original: &Path, link: &Path) -> Option<()> {
        match std::os::windows::fs::symlink_file(original, link) {
            Ok(()) => Some(()),
            Err(e) if e.raw_os_error() == Some(1314) => {
                eprintln!("skipping symlink assertion: privilege not held");
                None
            }
            Err(e) => panic!("symlink_file failed: {}", e),
        }
    }

    #[cfg(windows)]
    fn make_symlink_dir(original: &Path, link: &Path) -> Option<()> {
        match std::os::windows::fs::symlink_dir(original, link) {
            Ok(()) => Some(()),
            Err(e) if e.raw_os_error() == Some(1314) => {
                eprintln!("skipping symlink assertion: privilege not held");
                None
            }
            Err(e) => panic!("symlink_dir failed: {}", e),
        }
    }

    #[test]
    fn allows_paths_inside_root() {
        let t = TestDirs::new("inside");
        assert!(validate_path(&t.root, &t.root).is_ok());
        assert!(validate_path(&t.root, &t.root.join("config.json")).is_ok());
        assert!(validate_path(&t.root, &t.root.join("src").join("main.rs")).is_ok());
        // Non-existent file in an existing in-root directory is allowed.
        assert!(validate_path(&t.root, &t.root.join("src").join("new_file.rs")).is_ok());
        // Non-existent nested directories are allowed too.
        assert!(validate_path(&t.root, &t.root.join("a").join("b").join("c.txt")).is_ok());
    }

    #[test]
    fn rejects_plain_traversal() {
        let t = TestDirs::new("traversal");
        assert!(validate_path(
            &t.root,
            &t.root.join("src").join("..").join("..").join("outside.txt")
        )
        .is_err());
        assert!(validate_path(&t.root, &t.root.join("..").join("outside.txt")).is_err());
    }

    #[test]
    fn rejects_traversal_in_nonexistent_suffix() {
        let t = TestDirs::new("traversal_nonexistent");
        // The leaf and intermediate dir do not exist; the `..` segments climb
        // out of the root. Must be rejected regardless of which branch
        // (canonicalization or suffix scan) catches it.
        let target = t
            .root
            .join("ghost_dir")
            .join("..")
            .join("..")
            .join("evil_new_file.txt");
        assert!(validate_path(&t.root, &target).is_err());
    }

    #[test]
    fn rejects_absolute_path_outside_root() {
        let t = TestDirs::new("absolute");
        assert!(validate_path(&t.root, &t.base.join("outside.txt")).is_err());
        assert!(validate_path(&t.root, &t.base.join("outside_dir").join("secret.txt")).is_err());
        // A path that doesn't exist anywhere near the root.
        assert!(validate_path(&t.root, &t.base.join("outside_dir").join("nope.txt")).is_err());
    }

    #[test]
    fn rejects_sibling_directory_with_root_name_prefix() {
        // `…/project_evil` must not pass a check against root `…/project`
        // (guards against naive string-prefix matching).
        let t = TestDirs::new("sibling");
        let evil = t.base.join("project_evil");
        fs::create_dir_all(&evil).unwrap();
        fs::write(evil.join("file.txt"), "x").unwrap();
        assert!(validate_path(&t.root, &evil.join("file.txt")).is_err());
        assert!(validate_path(&t.root, &evil.join("new.txt")).is_err());
    }

    #[test]
    fn rejects_symlink_file_pointing_outside() {
        let t = TestDirs::new("symlink_file");
        let link = t.root.join("innocent.txt");
        if make_symlink_file(&t.base.join("outside.txt"), &link).is_none() {
            return;
        }
        assert!(validate_path(&t.root, &link).is_err());
    }

    #[test]
    fn rejects_symlinked_intermediate_directory() {
        let t = TestDirs::new("symlink_dir");
        let link_dir = t.root.join("vendor");
        if make_symlink_dir(&t.base.join("outside_dir"), &link_dir).is_none() {
            return;
        }
        // Existing file behind the symlinked directory.
        assert!(validate_path(&t.root, &link_dir.join("secret.txt")).is_err());
        // Non-existent leaf behind the symlinked directory: the nearest
        // existing parent IS the symlink, which canonicalizes outside.
        assert!(validate_path(&t.root, &link_dir.join("new_file.txt")).is_err());
    }

    #[test]
    fn allows_symlink_pointing_inside_root() {
        let t = TestDirs::new("symlink_inside");
        let link = t.root.join("alias.json");
        if make_symlink_file(&t.root.join("config.json"), &link).is_none() {
            return;
        }
        assert!(validate_path(&t.root, &link).is_ok());
    }

    #[cfg(windows)]
    mod windows {
        use super::*;

        #[test]
        fn verbatim_prefix_paths_are_validated() {
            let t = TestDirs::new("verbatim");
            let canonical_root = t.root.canonicalize().unwrap();
            // canonicalize() returns `\\?\C:\...` verbatim paths on Windows;
            // feeding them back in must work for in-root targets…
            let inside = canonical_root.join("config.json");
            assert!(inside.to_string_lossy().starts_with(r"\\?\"));
            assert!(validate_path(&t.root, &inside).is_ok());
            // …and still be rejected for outside targets.
            let outside = t.base.canonicalize().unwrap().join("outside.txt");
            assert!(outside.to_string_lossy().starts_with(r"\\?\"));
            assert!(validate_path(&t.root, &outside).is_err());
        }

        #[test]
        fn case_insensitive_paths_resolve_correctly() {
            let t = TestDirs::new("casefold");
            // NTFS is case-insensitive: an uppercase spelling of an in-root
            // file must be accepted (no false rejection from byte comparison)…
            let upper_inside = t.root.join("CONFIG.JSON");
            assert!(upper_inside.exists());
            assert!(validate_path(&t.root, &upper_inside).is_ok());
            // …and an uppercase spelling of an outside file must stay rejected.
            let upper_outside = t.base.join("OUTSIDE.TXT");
            assert!(validate_path(&t.root, &upper_outside).is_err());
        }

        #[test]
        fn short_8_3_names_resolve_to_long_names() {
            let t = TestDirs::new("shortname");
            let long_dir = t.root.join("longdirectoryname");
            fs::create_dir_all(&long_dir).unwrap();
            fs::write(long_dir.join("file.txt"), "x").unwrap();

            let short = t.root.join("LONGDI~1").join("file.txt");
            if !short.exists() {
                // 8.3 short-name generation is disabled on this volume.
                eprintln!("skipping 8.3 assertion: short names not generated");
                return;
            }
            // Short-name spelling of an in-root path must validate and
            // canonicalize inside the root.
            let resolved = validate_path(&t.root, &short).unwrap();
            assert!(resolved.starts_with(t.root.canonicalize().unwrap()));

            // Short-name spelling must not bypass the boundary either.
            let outside_long = t.base.join("longoutsidedirectory");
            fs::create_dir_all(&outside_long).unwrap();
            fs::write(outside_long.join("file.txt"), "x").unwrap();
            let outside_short = t.base.join("LONGOU~1").join("file.txt");
            if outside_short.exists() {
                assert!(validate_path(&t.root, &outside_short).is_err());
            }
        }

        #[test]
        fn rejects_alternate_data_stream_syntax() {
            let t = TestDirs::new("ads");
            fs::write(t.root.join("data.txt"), "x").unwrap();
            // Named stream (does not exist -> would fall into the
            // non-existent-path branch without the explicit guard).
            assert!(validate_path(&t.root, &t.root.join("data.txt:hidden")).is_err());
            // Default stream spelling of an existing file.
            assert!(validate_path(&t.root, &t.root.join("data.txt::$DATA")).is_err());
            // Stream on a directory.
            assert!(validate_path(&t.root, &t.root.join("src:stream")).is_err());
            // Stream on a non-existent file.
            assert!(validate_path(&t.root, &t.root.join("ghost.txt:stream:$DATA")).is_err());
        }

        #[test]
        fn rejects_reserved_device_names() {
            let t = TestDirs::new("devices");
            // `NUL` inside the root maps to the NUL device in the Win32
            // namespace; validation must fail closed rather than hand back a
            // path that opens a device.
            assert!(validate_path(&t.root, &t.root.join("NUL")).is_err());
            assert!(validate_path(&t.root, &t.root.join("CON")).is_err());
            // Reserved regardless of case, extension, or position.
            assert!(validate_path(&t.root, &t.root.join("nul.txt")).is_err());
            assert!(validate_path(&t.root, &t.root.join("COM1")).is_err());
            assert!(validate_path(&t.root, &t.root.join("src").join("lpt9.log")).is_err());
            // Names merely *containing* a device name stay allowed.
            assert!(validate_path(&t.root, &t.root.join("nullable.rs")).is_ok());
            assert!(validate_path(&t.root, &t.root.join("console.ts")).is_ok());
        }
    }
}
