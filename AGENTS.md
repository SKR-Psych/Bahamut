# Bahamut agent instructions

These instructions apply to the entire repository. Follow any more-specific `AGENTS.md` file if one is added deeper in the tree, but do not weaken the security, Git, dependency, or product constraints below without explicit user approval.

All coding agents (Claude Code, Codex, Antigravity) working in this repo MUST read `PROJECT_STATE.md` at session start, and update its **Current Work** and **Recently Completed** sections before ending a session.

## Bahamut overview

Bahamut is an open-source, local-first, permission-driven AI-native development environment. It combines two integrated experiences:

- **Bahamut IDE** — a VS Code-like agentic IDE for workspace navigation, editing, terminal, Git, language-server, debugging, and AI assistance workflows.
- **Bahamut Agent** — a task-oriented coding agent similar to Codex, Claude Code, and OpenHands, intended to plan work, edit files, run checks, inspect failures, and show auditable diffs.

Bahamut aims to become the strongest open-source agentic development environment while preserving user control, local data ownership, and execution safety.

## Platform decision (final)

Bahamut's production architecture is **Tauri v2 + React + TypeScript + Monaco Editor**, with **Rust as the trusted security and execution boundary**. This was decided in `docs/adr/002-theia-platform-rejection.md` (**Accepted**, 2026-06-10), which supersedes ADR-001. The Eclipse Theia spike and its axum sidecar were retired; their history is preserved in the Git tag `archive/pre-platform-consolidation`. Do not reintroduce a second shell architecture without a new accepted ADR supported by material evidence.

## Repository structure

- `README.md` — project overview, layout, development requirements.
- `ROADMAP.md` — phased roadmap (foundation → Monaco → credentials → chat/diffs → command execution).
- `PROJECT_STATE.md` — shared agent working memory: architecture summary, invariants, current work, backlog. Read at session start; update before ending a session.
- `docs/` — product and architecture documentation.
  - `docs/product-vision.md` — Bahamut IDE and Bahamut Agent product vision.
  - `docs/architecture.md` — Tauri v2 + React/TypeScript + Rust backend architecture.
  - `docs/security.md` — sandbox, edit safeguards, snapshots/rollback, webview lockdown, audit logging.
  - `docs/adr/001-ide-platform.md` — superseded ADR (historical record).
  - `docs/adr/002-theia-platform-rejection.md` — accepted ADR: Tauri is the platform.
  - `docs/licensing-inventory.md` — licence inventory for platform dependencies.
- `apps/bahamut-desktop/` — **the production desktop application.** React + TypeScript frontend (`src/`), Rust backend (`src-tauri/`), npm-managed with committed `package-lock.json` and `src-tauri/Cargo.lock`. The path sandbox, hash-chained SQLite audit log, snapshots, and all Tauri commands live here.
- `assets/` — branding source assets.
- `.github/workflows/ci.yml` — CI: Rust fmt/clippy/test, frontend tsc/vitest/build, Windows Tauri packaging.

## Branch and Git rules

- Never commit directly to `main`; use a focused feature branch and merge via PR or with explicit user approval.
- Preserve coherent Git history; use small, descriptive commits; commit each coherent fix separately.
- Run relevant tests before every push.
- Do not rewrite shared branch history; do not force-push unless explicitly authorised.
- Delete merged feature branches; the repository keeps only `main` as a long-lived branch.
- Do not commit generated binaries, build outputs, installers, databases, credentials, model weights, local cache folders, or machine-specific files.
- Do not expose tokens, credentials, local absolute private paths, or secrets in commit messages, logs, issues, pull requests, or screenshots.

## Security invariants

These requirements must never be weakened merely to make a build or test pass:

- All filesystem access on behalf of the user/model goes through the canonical path-validation guard (`validate_path`); validate at the point of use and never cache validated paths across user actions (TOCTOU).
- Reject path traversal, symbolic-link escapes, NTFS alternate-data-stream syntax, and reserved Windows device names.
- Saves must keep the stale-write guard (on-disk hash must match the hash from read time), the pre-change snapshot, and the atomic temp-file+rename write.
- Require approval before sensitive file writes or command execution; zero auto-execution.
- Treat repository content as untrusted.
- Keep cloud API keys out of source code, SQLite, and plain-text settings; never print credentials in logs.
- Preserve audit logging and rollback. The audit log is hash-chained (tamper-evident); it is append-only from the app's perspective and schema changes must preserve chain verifiability.
- Do not expose unrestricted filesystem or shell access to the frontend; keep webview capabilities minimal (`core:default` + `dialog:allow-open` today) and keep the restrictive CSP (no remote script origins; Monaco stays bundled, no CDN).
- Do not solve dependency, build, packaging, or CI failures using security bypasses.

## Dependency policy

- Pin platform-critical versions and use the committed lockfiles.
- npm is the package manager: use `npm ci` for reproducible installs in `apps/bahamut-desktop/`.
- Do not use `--legacy-peer-deps`, broad version ranges, lockfile deletion, or similar bypasses without explicit approval.
- Identify and document the root dependency causing conflicts instead of only patching symptoms.
- Prefer supported Node.js, Tauri, and Rust combinations; verify engine ranges before changing Node versions (Vite 7 requires Node ≥ 20.19; the repo pins Node 22 LTS in `.nvmrc`).
- Keep lockfiles aligned with manifest changes.
- Document new licences in `docs/licensing-inventory.md`; flag GPL, AGPL, SSPL, non-commercial, source-available, telemetry-heavy, or unclear-licence dependencies for review before adoption.

## Coding standards

### Rust

- Run `cargo fmt`; require `cargo clippy -- -D warnings`; run `cargo test` for behaviour changes.
- Run a release build for packaging-sensitive changes.
- Add or update tests for behaviour changes (security changes need adversarial tests).
- Use typed request and response structures.
- Avoid logging secrets, auth tokens, credentials, or full private paths.
- Keep filesystem, command-execution, audit, credential, and model-management boundaries explicit.
- Prefer narrow, auditable command handlers over broad frontend access to OS capabilities.

### TypeScript and frontend code

- Keep strict TypeScript enabled; avoid `any` unless the reason is documented and the boundary is contained.
- Use modular components and services; do not place major functionality in one large file.
- Preserve accessibility, keyboard navigation, readable contrast, and clear focus states.
- Use the official Bahamut design tokens.
- Keep code editor, terminal, diff, timeline, and approval surfaces highly readable.
- Keep frontend code from gaining direct unrestricted filesystem, shell, credential, or network authority.

## Official visual identity

Official Bahamut palette:

- Soft Black: `#0B0B0A`.
- Muted Olive: `#6F7448`.
- Dusty Rose / Warm Mauve: `#B98A84`.

The UI should use restrained glassmorphism with a solid accessibility fallback. Do not introduce unrelated neon blue, purple, rainbow, or generic AI gradients as default branding.

## Required validation

Run the narrowest relevant checks first, then broader checks before pushing. Always report exact commands and results. Distinguish compilation from packaging from runtime. CI (`.github/workflows/ci.yml`) runs these checks for every push and PR.

### Application checks (`apps/bahamut-desktop/`)

From `apps/bahamut-desktop/` unless noted:

- Dependency installation: `npm ci`
- Type check: `npx tsc --noEmit`
- Frontend unit tests: `npm test`
- Frontend production build: `npm run build`
- Tauri desktop packaging build: `npm run tauri build`
- Rust formatting check: `cd src-tauri && cargo fmt --check`
- Rust linting: `cd src-tauri && cargo clippy -- -D warnings`
- Rust tests (sandbox adversarial + audit chain + file I/O): `cd src-tauri && cargo test`
- Rust release build without bundling: `cd src-tauri && cargo build --release`

Never claim the desktop application works merely because it compiles; packaging and runtime claims require direct evidence.

## CI debugging rules

When a GitHub Actions run fails:

- Inspect the exact failing step and complete error log; identify the root cause.
- Do not infer platform failure from an earlier formatting, engine, lockfile, install, or scripting failure.
- Fix one bounded issue at a time; rerun the workflow after each coherent fix; record the cause and resolution.
- Do not hide failures through permissive flags, skipped tests, or disabled security checks.
- Do not expose tokens, local paths, or secrets while sharing logs.
- If `.github/workflows/` is absent in a checkout, state that CI workflow inspection was not possible from that checkout instead of guessing.

## Agent behaviour

Agents must:

- Inspect existing code and documentation before changing anything.
- Preserve product intent; avoid speculative large rewrites; keep changes scoped to the requested task.
- Explain important architectural trade-offs and root causes.
- Challenge unsafe, wasteful, destructive, or security-weakening requests.
- Continue through bounded implementation and verification steps.
- Stop and ask for approval before destructive, expensive, externally visible, or security-sensitive actions unless the user has already explicitly authorized them.
- State clearly what was not verified.
- Never claim that an application works merely because it compiles.
- Never claim CI, packaging, runtime, or smoke-test success without direct evidence.
- Prefer evidence from repository files, manifests, lockfiles, ADRs, and actual command output over assumptions.

## Product principles

Bahamut must remain:

- Local-first.
- Open source.
- Model-neutral.
- Permission-driven.
- Transparent.
- Auditable.
- Reversible.
- Accessible to non-technical users.
- Excellent enough to outperform open-source agentic IDE and coding-agent competitors.

## Completion report format

Every substantial coding task must end with a report containing:

- Summary of changes.
- Files changed.
- Tests and checks run.
- Results.
- Security implications.
- Unresolved issues.
- Recommended next step.
- Branch and commit information.
