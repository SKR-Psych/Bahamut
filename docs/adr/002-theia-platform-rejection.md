# ADR 002: Rejection of Eclipse Theia in Favor of Tauri + Monaco

## Status
**Decided: Reverted/Rejected**

## Context and Spike Findings

During the `feature/theia-platform-spike` migration spike, we attempted to build a branded Eclipse Theia Electron desktop application integrated with our local Rust sidecar backend. The following findings were established:

### 1. Native Dependency Compilation Blockers
- Eclipse Theia relies heavily on native Node.js binaries (such as `find-git-repositories` for Git tracking and `nsfw` for workspace file watching).
- Compiling these native packages on Windows requires specific Visual Studio Build Tools workloads (specifically **ClangCL Platform Toolset** and native headers), which are absent from standard developer machines.
- Furthermore, native Node-gyp configuration is highly sensitive to the local Python installation (e.g. Python 3.12+ removing `distutils` causes setup failures without manual `setuptools` configurations).
- **Implication**: Bundling and distributing a stable, cross-platform app using Theia introduces high maintenance burden and build pipeline vulnerability.

### 2. UI & Brand Customization Overhead
- Theia’s styling is driven by heavy, legacy CSS layouts and workspace widget manager structures. Applying Bahamut's visual specification (glassmorphic transparency overlays, custom status bar switches, brand color tokens) is constrained and requires overriding a large matrix of core theme variables, risking style regressions.
- Tauri + React provides absolute layout freedom, allowing us to implement glassmorphism easily using standard React components.

---

## Decision

We reject the pivot to Eclipse Theia and recommend **retaining the Tauri v2 + Monaco Editor shell** for Bahamut.

### Rationale
1.  **Zero Native Build Friction**: Tauri backend compiles directly to a standalone, dependency-free Rust executable. Node/C++ build tools (node-gyp, ClangCL) are not needed.
2.  **Branding Liberty**: Building the file explorer and tab manager in React allows us to implement Bahamut's visual identity (Muted Olive, Dusty Rose, Glassmorphism, and Solid fallback) without framework overrides.
3.  **Security Integration**: Security features (sandbox path-traversal guard, SQLite logs, processes) are natively bound to the Rust Tauri app context, eliminating IPC network overhead and token management.
