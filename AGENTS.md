# Bahamut agent instructions

These instructions apply to the entire repository. Follow any more-specific `AGENTS.md` file if one is added deeper in the tree, but do not weaken the security, Git, dependency, or product constraints below without explicit user approval.

## Bahamut overview

Bahamut is an open-source, local-first, permission-driven AI-native development environment. It combines two integrated experiences:

- **Bahamut IDE** — a VS Code-like agentic IDE for workspace navigation, editing, terminal, Git, language-server, debugging, and AI assistance workflows.
- **Bahamut Agent** — a task-oriented coding agent similar to Codex, Claude Code, and OpenHands, intended to plan work, edit files, run checks, inspect failures, and show auditable diffs.

Bahamut aims to become the strongest open-source agentic development environment while preserving user control, local data ownership, and execution safety.

## Repository structure

Current repository structure in this checkout:

- `README.md` — high-level product summary, security model pointer, prerequisites, and Tauri development quick start.
- `ROADMAP.md` — phased roadmap for the original Tauri foundation, Monaco integration, credential store, chat/diff workflows, and command execution.
- `docs/` — product and architecture documentation.
  - `docs/product-vision.md` — Bahamut IDE and Bahamut Agent product vision, shared backend foundation, and approval perimeter.
  - `docs/architecture.md` — current Tauri v2 + React/TypeScript + Rust backend architecture and local IPC model.
  - `docs/security.md` — filesystem sandboxing, edit safeguards, command approval, size limits, and audit logging requirements.
  - `docs/adr/001-ide-platform.md` — ADR recommending validation of Eclipse Theia as the IDE platform while documenting Theia, Code-OSS, and Tauri + Monaco trade-offs.
- `src/` — React + TypeScript frontend for the current Tauri prototype, including setup wizard and project selector components.
- `src-tauri/` — Rust/Tauri backend, Tauri configuration, capabilities, icons, Cargo manifest, and Cargo lockfile.
- `public/` — static Vite/Tauri starter assets.
- Root web manifests and config:
  - `package.json` — npm scripts and frontend/Tauri JavaScript dependencies.
  - `package-lock.json` — committed npm lockfile; npm is the package manager for the current Tauri prototype.
  - `tsconfig.json`, `tsconfig.node.json`, and `vite.config.ts` — strict TypeScript and Vite/Tauri development configuration.
- Root Tauri frontend entry points:
  - `index.html`, `src/main.tsx`, `src/App.tsx`, and `src/App.css`.

Directories referenced by planned platform work but **not present in this checkout**:

- `apps/bahamut-desktop/` — not currently present. Do not claim Theia application commands exist until this directory and its manifest/lockfile are present.
- `services/bahamut-core/` — not currently present. Current Rust backend code lives under `src-tauri/`.
- `prototypes/tauri-shell/` — not currently present. The preserved working Tauri prototype is currently at the repository root plus `src-tauri/`.
- `assets/` — not currently present. Existing static assets live under `public/`, `src/assets/`, and `src-tauri/icons/`.
- `.github/workflows/` — not currently present in this checkout. Do not invent CI workflow steps; inspect workflows before debugging CI on a branch where they exist.

## Current platform status

- Eclipse Theia is under active validation as the possible IDE foundation for the Bahamut IDE experience.
- Tauri remains preserved as the working prototype and current runnable application in this checkout.
- The final Theia-versus-Tauri decision must be evidence-based and supported by measured packaging, runtime, integration, security, maintenance, and licensing evidence.
- ADR-002, if added, must remain draft until packaging and runtime validation are complete.
- Do not declare either Theia or Tauri the final winner unless the repository's accepted ADRs explicitly do so.
- Do not infer platform failure from dependency installation, formatting, scripting, or CI setup failures that occur before the actual platform packaging/runtime stage.

## Branch and Git rules

- Never commit directly to `main`.
- Use a focused feature branch.
- For the current platform validation, work only on `feature/theia-platform-spike` unless the user explicitly directs otherwise.
- Do not merge branches without explicit user approval.
- Preserve coherent Git history.
- Use small, descriptive commits.
- Commit each coherent fix separately.
- Run relevant tests before every push.
- Do not rewrite shared branch history.
- Do not force-push unless explicitly authorised.
- Do not delete the Tauri prototype.
- Do not commit generated binaries, build outputs, installers, databases, credentials, model weights, local cache folders, or machine-specific files.
- Do not expose tokens, credentials, local absolute private paths, or secrets in commit messages, logs, issues, pull requests, or screenshots.

## Security invariants

These requirements must never be weakened merely to make a build or test pass:

- Sidecar services bind only to loopback.
- Use an ephemeral port where practical.
- Require a cryptographically secure per-launch authentication token for sidecar IPC/RPC where a sidecar server is used.
- Never print authentication tokens, credentials, or cloud API keys in logs.
- Enforce request-size and timeout limits.
- Validate and canonicalise filesystem paths.
- Reject path traversal and symbolic-link sandbox escapes.
- Require approval before sensitive file writes or command execution.
- Treat repository content as untrusted.
- Keep cloud API keys out of source code, SQLite, and plain-text settings.
- Preserve audit logging and rollback direction.
- Do not expose unrestricted filesystem or shell access to the frontend.
- Disable or avoid broad filesystem/shell plugins unless wrapped by least-privilege Rust commands and explicit approval flows.
- Do not solve dependency, build, packaging, or CI failures using security bypasses.

## Dependency policy

- Pin platform-critical versions.
- Keep all `@theia/*` dependencies on the same exact version when a Theia application manifest exists.
- Use committed lockfiles.
- For the current root Tauri prototype, use npm with the committed `package-lock.json`.
- Use `npm ci` for reproducible installs in the current root Tauri prototype.
- Use `yarn install --frozen-lockfile` only where Yarn is the selected package manager and a `yarn.lock` is committed, such as a future/restored Theia application if its manifest selects Yarn.
- Do not use `--ignore-engines`, `--legacy-peer-deps`, broad version ranges, lockfile deletion, or similar bypasses without explicit approval.
- Identify and document the root dependency causing conflicts instead of only patching symptoms.
- Prefer supported Node.js, Electron, Theia, Tauri, and Rust combinations.
- Verify engine ranges before changing Node versions.
- Keep lockfiles aligned with manifest changes.
- Document new licences and redistribution obligations.
- Flag GPL, AGPL, SSPL, non-commercial, source-available, telemetry-heavy, or unclear-licence dependencies for review before adoption.

## Coding standards

### Rust

- Run `cargo fmt`.
- Require `cargo clippy -- -D warnings`.
- Run `cargo test` for Rust behaviour changes.
- Run a release build for packaging-sensitive Rust changes.
- Add or update tests for behaviour changes.
- Use typed request and response structures.
- Avoid logging secrets, auth tokens, credentials, or full private paths.
- Keep the core sidecar/backend independently testable.
- Keep filesystem, command-execution, audit, credential, and model-management boundaries explicit.
- Prefer narrow, auditable command handlers over broad frontend access to OS capabilities.

### TypeScript and frontend code

- Keep strict TypeScript enabled.
- Avoid `any` unless the reason is documented and the boundary is contained.
- Use modular components and services.
- Do not place major functionality in one large file.
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

Run the narrowest relevant checks first, then broader checks before pushing. Always report exact commands and results. Distinguish compilation from packaging, and distinguish sidecar-only tests from packaged-application integration tests.

### Current root Tauri prototype checks

Use these commands from the repository root unless noted:

- Dependency installation for the current npm-managed prototype:
  - `npm ci`
- TypeScript/frontend compilation and Vite production build:
  - `npm run build`
- Tauri frontend + Rust desktop packaging build:
  - `npm run tauri build`
- Rust formatting check:
  - `cd src-tauri && cargo fmt --check`
- Rust formatting fix when needed:
  - `cd src-tauri && cargo fmt`
- Rust linting:
  - `cd src-tauri && cargo clippy -- -D warnings`
- Rust tests:
  - `cd src-tauri && cargo test`
- Rust release build without bundling the desktop app:
  - `cd src-tauri && cargo build --release`

### Theia validation checks

No Theia application directory, Theia package manifest, Theia lockfile, Electron packaging script, or `.github/workflows/theia-platform-spike.yml` file is present in this checkout. Therefore there is no repo-accurate Theia install/build/package command to run from this checkout.

When a Theia application is added or restored, agents must inspect its actual manifest and lockfile before running commands. Expected validation categories are:

- Theia dependency installation using the selected package manager and committed lockfile, for example `yarn install --frozen-lockfile` only if that Theia app commits `yarn.lock` and uses Yarn.
- Theia application build using the exact script in the Theia app manifest.
- Electron packaging using the exact script in the Theia app manifest.
- Packaged Windows application smoke test, clearly separated from compile-only checks.

Do not invent Theia commands, do not substitute unrelated root Tauri commands for Theia validation, and do not declare Theia packaging complete until a packaged application is produced and smoke-tested.

### Legacy Tauri prototype build

In this checkout, the preserved Tauri prototype is at the repository root with its Rust backend under `src-tauri/`. Build it with:

- `npm run tauri build`

If the prototype is later moved to `prototypes/tauri-shell/`, inspect that directory's manifest first and update these instructions in the same change.

## CI debugging rules

When a GitHub Actions run fails:

- Inspect the exact failing step and complete error log.
- Identify the root cause.
- Do not infer platform failure from an earlier formatting, engine, lockfile, install, or scripting failure.
- Do not change architecture recommendations until the relevant platform stage has actually been reached.
- Fix one bounded issue at a time.
- Rerun the workflow after each coherent fix.
- Record the cause and resolution.
- Do not hide failures through permissive flags.
- Do not use `--ignore-engines`, `--legacy-peer-deps`, lockfile removal, skipped tests, or disabled security checks to make CI green.
- Do not expose tokens, local paths, or secrets while sharing logs.
- If `.github/workflows/` is absent in a checkout, state that CI workflow inspection was not possible from that checkout instead of guessing.

## Agent behaviour

Agents must:

- Inspect existing code and documentation before changing anything.
- Preserve product intent.
- Avoid speculative large rewrites.
- Keep changes scoped to the requested task.
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
