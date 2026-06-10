# PROJECT_STATE.md — Shared Agent Working Memory

This file is the shared working memory for ALL coding agents (Claude Code,
Codex, Antigravity). Read it at session start. Update **Current Work** and
**Recently Completed** before ending a session. Keep this file under ~200
lines — prune old entries rather than letting it grow.

## Architecture Summary

Bahamut is a local-first, AI-native development environment with ONE
production architecture (ADR-002, **Accepted** 2026-06-10): **Tauri v2 +
React/TypeScript/Vite + Monaco**, with Rust as the trusted security and
execution boundary.

- `apps/bahamut-desktop/` — the production desktop application. Frontend in
  `src/` (React, bundled Monaco — no CDN), Rust backend in `src-tauri/`
  (path sandbox, hash-chained SQLite audit log, snapshots, all commands).
- `docs/` — architecture, security model, ADRs, product vision, licensing.
- `.github/workflows/ci.yml` — frontend tsc/vitest/build (Linux), Rust
  fmt/clippy/test (Linux), Windows `tauri build` packaging job.
- The Theia spike (`apps/bahamut-desktop` Theia variant) and axum sidecar
  (`services/bahamut-core`) were **retired** per ADR-002. History lives in
  the Git tag `archive/pre-platform-consolidation`.

### Tauri command surface (`apps/bahamut-desktop/src-tauri`)

| Command | File | What it does |
| --- | --- | --- |
| `set_project_root(path)` | `commands/security.rs` | Validates dir exists, canonicalizes, stores in `AppState.project_root`, audit-logs. |
| `check_file_in_sandbox(path)` | `commands/security.rs` | Runs `validate_path(root, path)`. |
| `list_project_files()` | `commands/files.rs` | Filtered tree: skips `.git`, `node_modules`, `target`, `dist`/`build`/`out`, binary extensions, files > configurable limit; capped depth/entries. |
| `read_project_file(path)` | `commands/files.rs` | Sandbox-validated read; rejects binary (NUL/invalid UTF-8) and oversized files; returns content + SHA-256. Denials audited. |
| `save_project_file(path, content, expected_hash)` | `commands/files.rs` | Revalidates at point of use; refuses stale writes (on-disk hash must equal `expected_hash`); pre-change snapshot to SQLite; atomic temp-file+rename write; audited. |
| `rollback_file_snapshot(snapshot_id)` | `commands/files.rs` | Restores a snapshot (revalidated against the CURRENT root); snapshots current content first (undo); atomic write; audited. |
| `list_file_snapshots(path)` | `commands/files.rs` | Newest-first snapshot metadata for one validated path. |
| `create_project_file(path)` / `create_project_folder(path)` | `commands/fileops.rs` | Validated create inside the sandbox; refuses overwrite (atomic `create_new`); audited (success/denied/failure). |
| `rename_project_path(from, to)` | `commands/fileops.rs` | Both endpoints validated; destination must not exist; refuses renaming the root; audited. |
| `delete_project_path(path)` | `commands/fileops.rs` | Recoverable delete: moves to `<app-data>/trash/<stamp>-<name>` and snapshots text files to SQLite (`pre-delete`) first; refuses deleting the root; audited. |
| `search_project(options)` | `commands/search.rs` | Bounded project-wide search (tree exclusions, configurable size cap, ≤20k files, ≤500 results, ≤30s); case/word/regex modes; async on a blocking thread. |
| `cancel_project_search()` | `commands/search.rs` | Advances the generation counter; any in-flight search aborts. |
| `get_app_settings` / `update_app_settings` / `reset_app_settings` | `commands/settings.rs` | Validated settings (size limits 1 KiB–50 MiB, theme whitelist, UI prefs) in the settings table; updates audited; no credentials ever stored here. |
| `get_snapshot_content(snapshot_id)` | `commands/files.rs` | Snapshot content for the diff view; stored path revalidated against the CURRENT root. |
| `get_hardware_info()` | `commands/system.rs` | RAM/CPU via `sysinfo`, GPU via `wmic`; VRAM is mocked (8 GB). |
| `check_ollama_status()` | `commands/system.rs` | GET `http://localhost:11434/api/tags` (2 s timeout). |
| `get_audit_logs()` | `database/mod.rs` | Last 100 audit rows (incl. seq + entry_hash), newest first. |
| `verify_audit_chain()` | `database/mod.rs` | Walks the audit hash chain; reports valid/first-broken-row. |

### Path validation (`validate_path` in `commands/security.rs`)

Canonicalizes the project root and the target; for non-existent targets,
canonicalizes the nearest existing parent and rejects `..` components in the
remaining suffix. Accepts only paths whose canonical form is inside the
canonical root. Symlinks resolve via canonicalization, so links pointing
outside the root are rejected. On Windows it also rejects NTFS
alternate-data-stream syntax (`:` in any component) and reserved DOS device
names (`NUL`, `CON`, `COM1`…). **TOCTOU note**: validation is path-based; the
returned path must be used immediately and never cached — every file I/O
command revalidates at the point of use.

### Audit log + snapshots (`database/mod.rs`)

SQLite at `%APPDATA%/Bahamut/bahamut.db` (`audit_logs`, `settings`,
`snapshots` tables). Audit rows are hash-chained: `seq` (monotonic),
`prev_hash`, `entry_hash = SHA-256(seq || prev_hash || canonical JSON
payload)` from a fixed genesis constant; the chain head is mirrored in
`settings` so tail deletion is detectable. Legacy tables migrate in place.
Tamper *evidence*, not prevention. Snapshots store pre-change file content
(path, content, content_hash, created_at); the configurable size limit lives
in `settings.max_file_size_bytes` (default 2 MiB).

### Security config (hardened 2026-06-10)

`tauri.conf.json`: identifier `com.samirahman.bahamut`, restrictive CSP
(script-src 'self', connect-src limited to Tauri IPC; dev variant allows the
Vite HMR socket). Capabilities: `core:default` + `dialog:allow-open` only.
No fs/shell/opener plugins.

## Invariants — no agent may break these

1. All file I/O on behalf of the model/agent goes through `validate_path`.
   No direct `fs` access to user paths from command handlers. Validated
   paths must not be cached across user actions; re-validate at the point of
   use (TOCTOU).
2. No command executes without explicit user approval (zero auto-execution).
3. No secrets (tokens, API keys, credential material) in logs, prompts,
   commits, or the audit DB.
4. Every change under a Rust crate keeps `cargo fmt --check`,
   `cargo clippy -- -D warnings`, and `cargo test` passing.
5. The audit log is append-only from the app's perspective; never rewrite or
   delete rows. Schema changes must preserve chain verifiability.
6. Saves must keep the stale-write guard (on-disk hash check), pre-change
   snapshot, and atomic write. Never write user files in place.

## Current Work

- (none — platform consolidation completed 2026-06-10)

## Recently Completed

- **2026-06-10** (Claude Code): IDE usability milestone. Multi-tab Monaco
  editing (per-tab models/undo, dirty markers, close confirmations, keyboard
  nav), secure create/rename/delete (`commands/fileops.rs`, recoverable
  trash + pre-delete snapshots), Rust project-wide search with limits and
  cancellation (`commands/search.rs`), validated settings commands
  (`commands/settings.rs`), snapshot UX with operation labels + Monaco diff
  modal, in-app Bahamut logo (header/welcome/empty states, derived 256px
  asset). Frontend test infra: vitest + jsdom + testing-library (39 tests);
  Rust suite at 52 tests.
- **2026-06-10** (Claude Code): application + installer branding. Tauri icon
  set (ico/icns/PNGs/Store logos) generated from
  `assets/branding/source/Bahamut Logo no bg no title.png` via the
  repeatable `npm run icons` script; tauri.conf.json gained publisher,
  short/longDescription, and the NSIS installerIcon. Regenerate icons only
  via `npm run icons` — never hand-edit `src-tauri/icons/`.
- **2026-06-10** (Claude Code): platform consolidation. ADR-002 Accepted
  (Tauri), ADR-001 superseded; Theia spike + axum sidecar retired (history
  in tag `archive/pre-platform-consolidation`); Tauri app moved
  `prototypes/tauri-shell` → `apps/bahamut-desktop`; hardened identifier/
  CSP/capabilities; implemented vertical slice (filtered tree, Monaco,
  hash-checked atomic save with snapshots, rollback, audit); docs + CI
  consolidated; branches reduced to `main`.

## Known Issues / Next Up (prioritised)

1. Trash management: deletes accumulate in `<app-data>/trash` with no purge
   or in-app restore UI yet (recovery is manual or via pre-delete
   snapshots). Cross-volume deletes are refused (trash move uses rename).
2. Project-wide replace is intentionally not implemented (search-only this
   milestone); revisit with the Phase 4 diff/approval flow.
3. Child-process sandboxing / environment scrubbing for approved commands
   (Roadmap Phase 5).
4. Prompt-injection flagging in the approval UI (highlight suspicious
   instructions inside file/diff content before the user approves).
5. VRAM detection is mocked in `get_hardware_info`; model download in
   `SetupWizard.tsx` is simulated, not a real Ollama pull (Phase 4).
6. Packaged-app smoke test in CI (currently CI verifies build/packaging,
   not runtime behaviour).
