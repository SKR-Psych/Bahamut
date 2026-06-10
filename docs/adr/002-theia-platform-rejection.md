# ADR 002: Rejection of Eclipse Theia in Favor of Tauri + Monaco

## Status

**Accepted** — 2026-06-10. Supersedes [ADR-001](001-ide-platform.md).

Bahamut's production architecture is **Tauri v2 + React + TypeScript + Monaco
Editor**, with Rust as the trusted security and execution boundary.

## Context

ADR-001 recommended validating Eclipse Theia as the desktop platform, with the
existing Rust security code re-packaged as a loopback sidecar. The
`feature/theia-platform-spike` branch carried out that validation: a branded
Theia/Electron application manifest (`apps/bahamut-desktop/`, pinned
`@theia/*` 1.49.0 + Electron 28.2.0 with a committed `yarn.lock`), a
token-authenticated axum sidecar (`services/bahamut-core/`), and a dedicated
Windows CI workflow (`theia-platform-spike.yml`).

## Spike Evidence

### 1. No packaged Theia application was ever produced or launched

This is the decisive finding. The spike CI workflow stopped at
`yarn build` (TypeScript + `theia build` webpack compile). The `yarn package`
(`theia package --mode=electron`) step was **never executed in CI**, and the
workflow's "smoke test" job only verified that the Rust sidecar `.exe` had
been copied into a *source* artifact folder — it never started, installed, or
exercised a packaged desktop application. Platform validation therefore never
reached the packaging/runtime stage, and no evidence exists that a
distributable Bahamut-on-Theia binary can be produced, let alone launched.

### 2. Native dependency and packaging burden

- Theia depends on native Node.js modules (`nsfw` for file watching,
  `find-git-repositories` for Git tracking) that must be compiled per
  platform with node-gyp.
- On Windows this requires specific Visual Studio Build Tools workloads
  (ClangCL platform toolset, native headers) absent from standard developer
  machines, and node-gyp is sensitive to the local Python installation
  (Python 3.12+ removed `distutils`, breaking setup without manual
  `setuptools` work).
- Distributing a stable cross-platform Theia app therefore carries a
  permanently fragile native build pipeline, plus the Electron runtime
  (~100+ MB) and a large pinned `@theia/*` dependency matrix to keep in
  lockstep.
- The Tauri shell, by contrast, compiles to a single standalone Rust
  executable using the OS webview; `npm run tauri build` produced a working
  Windows binary with no native Node toolchain at all.

### 3. Security architecture

- Under Theia, every sensitive operation would cross a localhost HTTP
  boundary to the sidecar, requiring per-launch token management, request
  limits, and timeout policy — all re-implemented and re-audited
  (`services/bahamut-core` had **no** path validation or audit logging when
  the spike ended).
- Under Tauri, the security perimeter already exists and is tested in-process:
  the canonical path sandbox (with adversarial tests covering symlink
  escapes, NTFS alternate data streams, reserved DOS device names, 8.3 short
  names, and verbatim paths) and the hash-chained SQLite audit log live in
  the Tauri Rust backend with no network surface.

### 4. Visual customisation

Theia theming is driven by a large matrix of legacy CSS variables and widget
manager structures; applying Bahamut's visual specification (glassmorphism
with a solid accessibility fallback, brand tokens) required broad overrides
with regression risk. Tauri + React gives unrestricted layout and styling.

### 5. Maintenance cost vs. rebuild cost (the accepted trade-off)

Theia's genuine advantage is out-of-box IDE features: workspace tree,
terminals, tabs, Git UI, debugging, LSP, and `.vsix` extension compatibility.
Rejecting Theia means Bahamut rebuilds the IDE features it needs (file tree,
editor tabs, terminal, diff viewer) on Monaco. We accept this cost because:

- Bahamut's product is a *permission-driven agentic surface*, not a general
  IDE; it needs a curated, auditable feature set rather than full IDE parity.
- Each rebuilt feature passes through the narrow Rust command boundary by
  construction, which the security model requires anyway.
- The working implementation is already on Tauri; the Theia path required
  migrating and re-securing everything before adding any product value.

## Decision

1. **Adopt Tauri v2 + React + TypeScript + Monaco Editor** as Bahamut's one
   production architecture. The Tauri application moves to
   `apps/bahamut-desktop/` as the production application.
2. **Retire the Theia spike.** The spike's manifest, branding shell, and
   workflow are removed from the active tree; their history is preserved in
   the Git tag `archive/pre-platform-consolidation` and the measured findings
   are recorded in this ADR.
3. **Retire the axum sidecar** (`services/bahamut-core/`). It existed solely
   to serve a Theia/Electron frontend. Its useful behaviour (Ollama status
   probing, loopback discipline) already exists in, or is unnecessary under,
   the in-process Tauri backend. No unique security logic existed to migrate.
4. **ADR-001 is superseded** by this ADR.

## Consequences

- One architecture, one security implementation, one dependency toolchain
  (npm + cargo); the Yarn/Theia/Electron toolchain leaves the repository.
- IDE conveniences (terminal, Git UI, LSP, extensions) must be built
  deliberately, feature by feature, behind the Rust approval perimeter
  (see ROADMAP).
- Re-opening the Theia question would require new material evidence — at
  minimum a reproducibly packaged, launchable Theia application with the
  full security perimeter implemented behind it.
