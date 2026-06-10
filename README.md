# Bahamut

Bahamut is an open-source, local-first, permission-driven AI-native development
environment. It combines an agentic IDE (**IDE mode**) and a task-oriented
coding agent (**Agent mode**) in one desktop application.

## Architecture

Bahamut is built on **Tauri v2 + React + TypeScript + Monaco Editor**
(see [ADR-002](docs/adr/002-theia-platform-rejection.md), Accepted):

- **Rust is the trusted security and execution boundary.** All sensitive
  filesystem and command operations pass through narrow, audited Rust
  commands — the webview holds no filesystem or shell permissions.
- Every file path is canonicalised and validated against the user-selected
  project root (symlink-escape, NTFS alternate-data-stream, and reserved
  device-name protections included), revalidated at the point of use.
- Saves verify the original file hash, store a pre-change snapshot, and write
  atomically; snapshots can be restored (and the restore undone).
- Every save, rollback, and denied attempt is recorded in a hash-chained,
  tamper-evident SQLite audit log (`verify_audit_chain`).
- Bahamut remains local-first, model-neutral, permission-driven, and auditable.

## Repository Layout

- **`apps/bahamut-desktop/`** — the production desktop application.
  React + TypeScript frontend (`src/`), Rust backend (`src-tauri/`),
  npm-managed with committed `package-lock.json` and `src-tauri/Cargo.lock`.
- **`docs/`** — product vision, architecture, security model, ADRs, licensing.
- **`assets/`** — branding source assets.
- **`.github/workflows/`** — CI (frontend type-check/tests/build, Rust
  fmt/clippy/test, Windows Tauri packaging).

The earlier Eclipse Theia spike and its Rust sidecar were retired per ADR-002;
their history is preserved in the `archive/pre-platform-consolidation` tag.

## Development Requirements

- **Node.js**: `22.12.0` or later (see `.nvmrc`; Vite 7 requires ≥ 20.19)
- **npm**: lockfile-driven installs via `npm ci`
- **Rust**: stable toolchain (MSVC on Windows)

## Building and Testing

From `apps/bahamut-desktop/`:

```bash
npm ci             # reproducible dependency install
npx tsc --noEmit   # type check
npm test           # frontend unit tests (vitest)
npm run build      # production frontend build
npm run tauri dev  # run the desktop app in development
npm run tauri build  # package the desktop app
```

From `apps/bahamut-desktop/src-tauri/`:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test         # includes adversarial sandbox + audit-chain tests
```
