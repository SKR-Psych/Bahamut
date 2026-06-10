# PROJECT_STATE.md — Shared Agent Working Memory

This file is the shared working memory for ALL coding agents (Claude Code,
Codex, Antigravity). Read it at session start. Update **Current Work** and
**Recently Completed** before ending a session. Keep this file under ~200
lines — prune old entries rather than letting it grow.

## Architecture Summary

Bahamut is a local-first, AI-native development environment. The repo is a
monorepo, currently mid-decision between two shells (see ADR-001 vs ADR-002,
the latter a draft recommending **retaining the Tauri shell**):

- `prototypes/tauri-shell/` — Tauri v2 (Rust backend) + React/TypeScript/Vite
  frontend. **This is where the real security code lives today**: the path
  sandbox, the SQLite audit log, and all Tauri commands.
- `services/bahamut-core/` — Rust axum sidecar (built for the Theia spike).
  Binds 127.0.0.1 on an ephemeral port, requires the `X-Bahamut-Auth` header
  matching the `BAHAMUT_AUTH_TOKEN` env var, exits when stdin closes. Routes:
  `/v1/health`, `/v1/ollama/status`, `/v1/sandbox`. It has **no** path
  validation or audit-log implementation yet.
- `apps/bahamut-desktop/` — Eclipse Theia/Electron shell spike (branding,
  agent panel widget, HTTP client for the sidecar). ADR-002 recommends
  rejecting this direction.
- `docs/` — architecture, security model, ADRs, product vision.
- `.github/workflows/` — `ci.yml` (push/PR, Linux) and
  `theia-platform-spike.yml` (Windows build of the Theia spike branch).

### Tauri command surface (`prototypes/tauri-shell/src-tauri`)

| Command | File | What it does |
| --- | --- | --- |
| `set_project_root(path)` | `commands/security.rs` | Validates dir exists, canonicalizes, stores it in `AppState.project_root`, audit-logs the selection. |
| `check_file_in_sandbox(path)` | `commands/security.rs` | Runs `validate_path(root, path)`; Ok(true) if inside the sandbox, Err otherwise. |
| `get_hardware_info()` | `commands/system.rs` | RAM/CPU via `sysinfo`, GPU via `wmic` on Windows; VRAM is mocked (8 GB). |
| `check_ollama_status()` | `commands/system.rs` | GET `http://localhost:11434/api/tags` (2 s timeout); returns running flag + model names. |
| `get_audit_logs()` | `database/mod.rs` | Returns last 100 audit rows (JSON) ordered newest first. |

### Path validation (`validate_path` in `commands/security.rs`)

Canonicalizes the project root and the target; for non-existent targets,
canonicalizes the nearest existing parent and rejects `..` components in the
remaining (non-existent) suffix. Accepts only paths whose canonical form is
inside the canonical root. Symlinks are resolved by canonicalization, so a
link pointing outside the root is rejected. **TOCTOU note**: validation is
path-based; the returned path must be used immediately and never cached —
re-validate at open time when file I/O commands are added (see Invariants).

### Audit log (`database/mod.rs`)

SQLite at `%APPDATA%/Bahamut/bahamut.db` (`audit_logs` + `settings` tables).
`log_action()` inserts `(action_type, details, status, error)` rows with a
SQLite `CURRENT_TIMESTAMP`. Rows currently carry **no tamper evidence**
(hash chaining is planned — see Known Issues).

### Diffs

Diff proposal/apply (Monaco diff viewer, hash-checked pre-apply verification)
is **documented in docs/ but not implemented yet** (Roadmap Phase 4).

## Invariants — no agent may break these

1. All file I/O on behalf of the model/agent goes through the path-validation
   guard (`validate_path`). No direct `fs` access to user paths from command
   handlers. Validated paths must not be cached across user actions;
   re-validate at the point of use (TOCTOU).
2. No command executes without explicit user approval (zero auto-execution).
3. No secrets (tokens, API keys, credential material) in logs, prompts,
   commits, or the audit DB.
4. Every change under a Rust crate keeps `cargo fmt --check`,
   `cargo clippy -- -D warnings`, and `cargo test` passing.
5. The audit log is append-only from the app's perspective; never rewrite or
   delete rows. Schema changes must preserve chain verifiability.

## Current Work

- **Claude Code** (2026-06-10): security hardening pass — CI scaffolding,
  adversarial sandbox tests, hash-chained audit log (Steps 3–5 of the
  hardening plan).

## Recently Completed

- (none yet — add dated entries here, newest first, max 3 lines each)

## Known Issues / Next Up (prioritised)

1. Audit log has no tamper evidence — add hash chaining + verification
   (in progress, see Current Work).
2. Decide ADR-002 (Theia rejection is still Draft); consolidate on one shell
   and retire the unused one.
3. File read/write Tauri commands (Roadmap Phase 2) — must use the sandbox
   guard and re-validate at open time.
4. `tauri.conf.json` still has default identifiers (`com.sami.tauri-app`) and
   `csp: null`; needs hardening before any release.
5. VRAM detection is mocked in `get_hardware_info`.
6. Model download in `SetupWizard.tsx` is simulated, not a real Ollama pull.
