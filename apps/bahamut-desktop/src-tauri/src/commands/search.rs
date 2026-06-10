//! Project-wide text search, implemented in Rust so the frontend never walks
//! the filesystem itself. The walk honours the same exclusions as the project
//! tree (.git, node_modules, target, build outputs, binary extensions) plus a
//! configurable searched-file size limit, and is bounded by file-count,
//! result-count, and time limits. A generation counter supports cancellation:
//! starting a new search or invoking the cancel command aborts a running one.

use crate::commands::files::{has_binary_extension, is_ignored_dir};
use crate::AppState;
use regex::RegexBuilder;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::State;

/// Hard caps, independent of user-configurable settings.
const MAX_FILES_SCANNED: u64 = 20_000;
const MAX_TOTAL_RESULTS: usize = 500;
const MAX_RESULTS_PER_FILE: usize = 50;
const MAX_TIMEOUT_MS: u64 = 30_000;
const MAX_PREVIEW_CHARS: usize = 240;
const MAX_WALK_DEPTH: usize = 32;

fn default_max_results() -> usize {
    300
}
fn default_timeout_ms() -> u64 {
    10_000
}

#[derive(Debug, Deserialize)]
pub struct SearchOptions {
    pub query: String,
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default)]
    pub whole_word: bool,
    #[serde(default)]
    pub regex: bool,
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct SearchMatch {
    pub line: usize,
    pub column: usize,
    pub preview: String,
}

#[derive(Debug, Serialize)]
pub struct FileSearchResult {
    pub path: String,
    pub name: String,
    pub matches: Vec<SearchMatch>,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub files: Vec<FileSearchResult>,
    pub total_matches: usize,
    pub files_scanned: u64,
    pub truncated: bool,
    pub timed_out: bool,
    pub cancelled: bool,
}

fn truncate_preview(line: &str) -> String {
    let trimmed = line.trim_end();
    if trimmed.chars().count() <= MAX_PREVIEW_CHARS {
        trimmed.to_string()
    } else {
        let cut: String = trimmed.chars().take(MAX_PREVIEW_CHARS).collect();
        format!("{}…", cut)
    }
}

/// Pure search engine; the Tauri wrapper supplies root, size limit, and the
/// cancellation generation.
pub fn search_core(
    root: &Path,
    max_file_size: u64,
    opts: &SearchOptions,
    generation: &AtomicU64,
    my_generation: u64,
) -> Result<SearchResponse, String> {
    if opts.query.is_empty() {
        return Err("Search query is empty".to_string());
    }

    let pattern = if opts.regex {
        opts.query.clone()
    } else {
        regex::escape(&opts.query)
    };
    let pattern = if opts.whole_word {
        format!(r"\b(?:{})\b", pattern)
    } else {
        pattern
    };
    let matcher = RegexBuilder::new(&pattern)
        .case_insensitive(!opts.case_sensitive)
        .size_limit(1 << 20)
        .build()
        .map_err(|e| format!("Invalid search pattern: {}", e))?;

    let max_results = opts.max_results.clamp(1, MAX_TOTAL_RESULTS);
    let timeout = Duration::from_millis(opts.timeout_ms.min(MAX_TIMEOUT_MS));
    let started = Instant::now();

    let canonical_root = root
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize project root: {}", e))?;

    let mut response = SearchResponse {
        files: Vec::new(),
        total_matches: 0,
        files_scanned: 0,
        truncated: false,
        timed_out: false,
        cancelled: false,
    };

    // Iterative depth-first walk (dirs sorted for stable result order).
    let mut stack: Vec<(PathBuf, usize)> = vec![(canonical_root, 0)];
    'walk: while let Some((dir, depth)) = stack.pop() {
        if depth > MAX_WALK_DEPTH {
            response.truncated = true;
            continue;
        }
        let read_dir = match std::fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        let mut entries: Vec<_> = read_dir.flatten().collect();
        entries.sort_by_key(|e| e.file_name().to_string_lossy().to_lowercase());

        for entry in entries {
            if generation.load(Ordering::Relaxed) != my_generation {
                response.cancelled = true;
                break 'walk;
            }
            if started.elapsed() >= timeout {
                response.timed_out = true;
                break 'walk;
            }

            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let file_type = match entry.file_type() {
                Ok(t) => t,
                Err(_) => continue,
            };
            if file_type.is_dir() {
                if !is_ignored_dir(&name) {
                    stack.push((path, depth + 1));
                }
                continue;
            }
            if !file_type.is_file() || has_binary_extension(&path) {
                continue;
            }
            if let Ok(meta) = entry.metadata() {
                if meta.len() > max_file_size {
                    continue;
                }
            }
            if response.files_scanned >= MAX_FILES_SCANNED {
                response.truncated = true;
                break 'walk;
            }
            response.files_scanned += 1;

            let bytes = match std::fs::read(&path) {
                Ok(b) => b,
                Err(_) => continue,
            };
            if bytes.contains(&0) {
                continue;
            }
            let text = match String::from_utf8(bytes) {
                Ok(t) => t,
                Err(_) => continue,
            };

            let mut matches = Vec::new();
            for (idx, line) in text.lines().enumerate() {
                if let Some(m) = matcher.find(line) {
                    matches.push(SearchMatch {
                        line: idx + 1,
                        column: line[..m.start()].chars().count() + 1,
                        preview: truncate_preview(line),
                    });
                    if matches.len() >= MAX_RESULTS_PER_FILE {
                        response.truncated = true;
                        break;
                    }
                    if response.total_matches + matches.len() >= max_results {
                        response.truncated = true;
                        break;
                    }
                }
            }
            if !matches.is_empty() {
                response.total_matches += matches.len();
                response.files.push(FileSearchResult {
                    path: path.to_string_lossy().to_string(),
                    name,
                    matches,
                });
                if response.total_matches >= max_results {
                    response.truncated = true;
                    break 'walk;
                }
            }
        }
    }

    Ok(response)
}

/// Async command: the walk runs on a blocking thread so the UI thread is
/// never blocked, and `cancel_project_search` can interleave.
#[tauri::command]
pub async fn search_project(
    state: State<'_, AppState>,
    options: SearchOptions,
) -> Result<SearchResponse, String> {
    let (root, max_file_size, generation, my_generation) = {
        let root_guard = state.project_root.lock().map_err(|_| "Mutex error")?;
        let root = root_guard
            .as_ref()
            .ok_or_else(|| "No project folder opened".to_string())?
            .clone();
        let conn_guard = state.db_conn.lock().map_err(|_| "Mutex error")?;
        let conn = conn_guard
            .as_ref()
            .ok_or_else(|| "Database connection not initialized".to_string())?;
        let max = crate::database::get_max_search_file_size(conn);
        // Starting a new search implicitly cancels any running one.
        let gen = state.search_generation.fetch_add(1, Ordering::SeqCst) + 1;
        (root, max, Arc::clone(&state.search_generation), gen)
    };

    tauri::async_runtime::spawn_blocking(move || {
        search_core(&root, max_file_size, &options, &generation, my_generation)
    })
    .await
    .map_err(|e| format!("Search task failed: {}", e))?
}

/// Aborts any in-flight search by advancing the generation counter.
#[tauri::command]
pub fn cancel_project_search(state: State<'_, AppState>) -> Result<(), String> {
    state.search_generation.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct TestEnv {
        base: PathBuf,
        root: PathBuf,
    }

    impl TestEnv {
        fn new(tag: &str) -> Self {
            let base =
                std::env::temp_dir().join(format!("bahamut_search_{}_{}", tag, std::process::id()));
            let _ = fs::remove_dir_all(&base);
            let root = base.join("project");
            fs::create_dir_all(root.join("src")).unwrap();
            fs::write(
                root.join("src").join("main.rs"),
                "fn main() {\n    println!(\"needle\");\n}\nlet needle_case = 1;\n",
            )
            .unwrap();
            fs::write(root.join("README.md"), "Needle in docs\nplain line\n").unwrap();
            TestEnv { base, root }
        }
    }

    impl Drop for TestEnv {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.base);
        }
    }

    fn opts(query: &str) -> SearchOptions {
        SearchOptions {
            query: query.to_string(),
            case_sensitive: false,
            whole_word: false,
            regex: false,
            max_results: default_max_results(),
            timeout_ms: default_timeout_ms(),
        }
    }

    fn run(root: &Path, o: &SearchOptions) -> SearchResponse {
        let generation = AtomicU64::new(1);
        search_core(root, 1024 * 1024, o, &generation, 1).unwrap()
    }

    #[test]
    fn finds_matches_with_line_numbers_grouped_by_file() {
        let t = TestEnv::new("basic");
        let resp = run(&t.root, &opts("needle"));
        assert_eq!(resp.files.len(), 2, "{:?}", resp.files);
        let main = resp
            .files
            .iter()
            .find(|f| f.name == "main.rs")
            .expect("main.rs in results");
        assert_eq!(main.matches[0].line, 2);
        assert!(main.matches[0].preview.contains("needle"));
        assert!(!resp.truncated && !resp.timed_out && !resp.cancelled);
    }

    #[test]
    fn respects_exclusions_and_size_limit() {
        let t = TestEnv::new("exclusions");
        fs::create_dir_all(t.root.join("node_modules").join("pkg")).unwrap();
        fs::write(
            t.root.join("node_modules").join("pkg").join("dep.js"),
            "needle\n",
        )
        .unwrap();
        fs::create_dir_all(t.root.join(".git")).unwrap();
        fs::write(t.root.join(".git").join("config"), "needle\n").unwrap();
        fs::write(t.root.join("image.png"), "needle").unwrap();
        fs::write(
            t.root.join("big.txt"),
            format!("needle\n{}", "x".repeat(4096)),
        )
        .unwrap();

        let generation = AtomicU64::new(1);
        let resp = search_core(&t.root, 1024, &opts("needle"), &generation, 1).unwrap();
        let names: Vec<&str> = resp.files.iter().map(|f| f.name.as_str()).collect();
        assert!(!names.contains(&"dep.js"), "node_modules excluded");
        assert!(!names.contains(&"config"), ".git excluded");
        assert!(!names.contains(&"image.png"), "binary extension excluded");
        assert!(!names.contains(&"big.txt"), "oversized file excluded");
        assert!(names.contains(&"main.rs"));
    }

    #[test]
    fn case_sensitivity_and_whole_word() {
        let t = TestEnv::new("modes");
        let mut o = opts("Needle");
        o.case_sensitive = true;
        let resp = run(&t.root, &o);
        assert_eq!(resp.files.len(), 1);
        assert_eq!(resp.files[0].name, "README.md");

        let mut o = opts("needle");
        o.whole_word = true;
        let resp = run(&t.root, &o);
        // "needle_case" must not match whole-word "needle".
        let main = resp.files.iter().find(|f| f.name == "main.rs").unwrap();
        assert_eq!(main.matches.len(), 1);
        assert_eq!(main.matches[0].line, 2);
    }

    #[test]
    fn regex_mode_and_invalid_pattern() {
        let t = TestEnv::new("regex");
        let mut o = opts(r"need\w+_case");
        o.regex = true;
        let resp = run(&t.root, &o);
        assert_eq!(resp.files.len(), 1);
        assert_eq!(resp.files[0].matches[0].line, 4);

        let mut bad = opts(r"[unclosed");
        bad.regex = true;
        let generation = AtomicU64::new(1);
        let err = search_core(&t.root, 1024 * 1024, &bad, &generation, 1).unwrap_err();
        assert!(err.contains("Invalid search pattern"), "{}", err);
    }

    #[test]
    fn result_cap_truncates() {
        let t = TestEnv::new("cap");
        let mut o = opts("needle");
        o.max_results = 1;
        let resp = run(&t.root, &o);
        assert_eq!(resp.total_matches, 1);
        assert!(resp.truncated);
    }

    #[test]
    fn zero_timeout_reports_timed_out() {
        let t = TestEnv::new("timeout");
        let mut o = opts("needle");
        o.timeout_ms = 0;
        let resp = run(&t.root, &o);
        assert!(resp.timed_out);
        assert_eq!(resp.total_matches, 0);
    }

    #[test]
    fn advanced_generation_cancels_search() {
        let t = TestEnv::new("cancel");
        let generation = AtomicU64::new(2); // already moved past my_generation=1
        let resp = search_core(&t.root, 1024 * 1024, &opts("needle"), &generation, 1).unwrap();
        assert!(resp.cancelled);
        assert_eq!(resp.total_matches, 0);
    }
}
