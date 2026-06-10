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
| `get_audit_logs()` | `database/mod.rs` | Returns last 100 audit rows (JSON, incl. seq + entry_hash) newest first. |
| `verify_audit_chain()` | `database/mod.rs` | Walks the audit hash chain; reports valid/first-broken-row for the UI. |

### Path validation (`validate_path` in `commands/security.rs`)

Canonicalizes the project root and the target; for non-existent targets,
canonicalizes the nearest existing parent and rejects `..` components in the
remaining (non-existent) suffix. Accepts only paths whose canonical form is
inside the canonical root. Symlinks are resolved by canonicalization, so a
link pointing outside the root is rejected. On Windows it also rejects NTFS
alternate-data-stream syntax (`:` in any component) and reserved DOS device
names (`NUL`, `CON`, `COM1`…, with or without extension) — both bypassed the
canonicalization check for non-existent targets (fixed 2026-06-10; the
device-name case was a real fail-open found by adversarial tests).
**TOCTOU note**: validation is path-based; the returned path must be used
immediately and never cached — re-validate at open time when file I/O
commands are added (see Invariants).

### Audit log (`database/mod.rs`)

SQLite at `%APPDATA%/Bahamut/bahamut.db` (`audit_logs` + `settings` tables).
Tamper-evident via hash chaining: each row stores `seq` (monotonic from 1),
`prev_hash`, and `entry_hash = SHA-256(seq || prev_hash || canonical JSON
payload)`, chained from a fixed genesis constant; the chain head is mirrored
in `settings` so tail deletion is detectable. `verify_chain()` /
`verify_audit_chain` report the first broken link. Legacy (pre-chain) tables
are migrated in place. This is tamper *evidence*, not prevention.

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

- (none — merge-conflict check complete on 2026-06-10)

## Recently Completed

- **2026-06-10** (Copilot): processed PR comments requesting merge-conflict
  resolution; fast-forwarded local branch to `db369eb` (which merged
  `origin/main`) and verified `origin/main` is already up to date with this
  branch (no remaining conflicts).
- **2026-06-10** (Claude Code): hash-chained the audit log (seq + SHA-256
  chain + head pointer), added `verify_audit_chain` command + 8 tests,
  updated docs/security.md. Verify: `cargo test` in tauri-shell crate.
- **2026-06-10** (Claude Code): adversarial sandbox tests (13) for
  `validate_path`; fixed real Windows bypasses (ADS syntax, reserved device
  names like `project\NUL`); documented TOCTOU limits. Verify: `cargo test`.
- **2026-06-10** (Claude Code): added `.github/workflows/ci.yml` (Linux:
  fmt/clippy/test for both Rust crates, npm ci + tsc for the frontend);
  fixed tauri-shell crate compile (sysinfo 0.30 `SystemExt` removal).

## Known Issues / Next Up (prioritised)

1. Snapshot-based rollback for applied file changes (undo an approved edit
   from a pre-change snapshot).
2. Child-process sandboxing / environment scrubbing for approved commands
   (strip secrets from env, constrain working dir and privileges).
3. Tauri capabilities lockdown: `tauri.conf.json` still has default
   identifier (`com.sami.tauri-app`) and `csp: null`; capabilities file is
   permissive default.
4. Prompt-injection flagging in the approval UI (highlight suspicious
   instructions inside file/diff content before the user approves).
5. Decide ADR-002 (Theia rejection is still Draft); consolidate on one shell
   and retire the unused one.
6. File read/write Tauri commands (Roadmap Phase 2) — must use the sandbox
   guard and re-validate at open time.
7. VRAM detection is mocked in `get_hardware_info`; model download in
   `SetupWizard.tsx` is simulated, not a real Ollama pull.
