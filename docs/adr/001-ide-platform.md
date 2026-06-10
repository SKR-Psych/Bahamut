# ADR 001: IDE Platform Selection

## Status

**Superseded by [ADR-002](002-theia-platform-rejection.md)** (2026-06-10).

The Theia recommendation below was validated by the
`feature/theia-platform-spike` work and rejected on the evidence recorded in
ADR-002: no packaged Theia application was ever produced or launched, the
native dependency and packaging burden was high, and the security perimeter
would have had to be rebuilt behind a localhost IPC boundary. Bahamut's
production architecture is Tauri v2 + React + TypeScript + Monaco Editor.
This document is retained as the record of the original comparison.

## Context and Requirements

Bahamut requires a robust desktop application packaging two distinct, integrated modes:
1.  **Bahamut IDE**: A full-featured development environment with file explorer, multi-tab editing, terminal, git integration, debugging, and Language Server Protocol (LSP) support.
2.  **Bahamut Agent**: A task-oriented autonomous agent that edits multiple files, runs builds/tests, and renders diffs.

Both modes must share project context, local/cloud model configurations, path-traversal sandboxes, command runtimes, and local SQLite audit logs. The application must support Windows first, allow custom visual branding (including glassmorphism), and maintain a low long-term code maintenance burden.

We compare three platforms to serve as the application shell:
1.  **Eclipse Theia**
2.  **Direct Code-OSS Fork**
3.  **Tauri + Custom Monaco Shell** (Current Phase 1 foundation)

---

## Platform Comparison

| Evaluation Metric | 1. Eclipse Theia | 2. Code-OSS Fork | 3. Tauri + Monaco |
| :--- | :--- | :--- | :--- |
| **Out-of-box IDE Features** | **High**: Includes workspace tree, git, terminal, tabs, debuggers, and language servers. | **High**: Complete VS Code capabilities. | **Low**: Explorer, terminals, and tab managers must be written from scratch. |
| **Extension Support** | **Yes**: Native compatibility with standard `.vsix` extensions via Open-VSX. | **Yes**: Native. | **No**: Custom plugin environment required. |
| **Custom UI & Branding** | **Excellent**: Built from the ground up to be highly customizable, modular, and white-labeled. | **Poor**: Monolithic; hard to customize main layout or inject custom Agent panels. | **Unrestricted**: Full custom React views and glassmorphic designs. |
| **Security & Sandbox** | **High**: Can restrict filesystem access by embedding a local Rust sidecar for path validation. | **Medium**: Relies on standard Node APIs. | **Very High**: Direct Rust backend and IPC sandbox controls. |
| **Licensing** | **EPL-2.0**: Highly permissive for rebranding and redistribution. | **MIT / Proprietary**: Forking Code-OSS requires stripping Microsoft trademarks and telemetry. | **MIT**: Free. |
| **Maintenance Burden** | **Low**: Theia core team handles IDE shell, terminal, and editor updates. | **High**: Syncing a custom Code-OSS fork with upstream is labor-intensive. | **Extremely High**: Rewriting file search, git UI, tabs, and terminal integrations. |

---

## Decision and Recommendation

We recommend **pivoting to Eclipse Theia** as the core desktop platform for Bahamut.

### Rationale
- **Development Speed**: Theia provides a complete VS Code-like desktop app out-of-the-box (workspace explorer, terminals, tabs, git, and debugger integration) saving months of custom UI engineering.
- **Modularity**: Theia is built specifically for customization. We can package Bahamut IDE and Bahamut Agent as a custom product distribution with branded glassmorphic layouts, and embed the Agent as a custom panel.
- **Extension Compatibility**: We can leverage existing VS Code extensions (LSPs, debuggers) out of the box.
- **Leveraging Rust Backend**: We will package our Rust security sandbox, SQLite database, and Ollama agent executor as a **local sidecar daemon** (communicating via JSON-RPC or WebSocket) packaged inside the Electron application bundle.

---

## Migration and Architectural Coexistence

```mermaid
graph TD
    subgraph TheiaApp [Theia Desktop Shell (Electron)]
        TheiaUI[Branded Theia UI & Extensions]
        AgentPanel[Custom Agent Panel]
    end

    subgraph RustSidecar [Rust Security & Agent Sidecar]
        RPC[RPC / WebSocket Server]
        FSGuard[File System Guard]
        DB[(SQLite DB)]
    end

    TheiaUI -->|User Inputs| AgentPanel
    AgentPanel -->|Query API / Execute| RPC
    RPC -->|Verify Sandbox Path| FSGuard
    RPC -->|Audit Logs| DB
    FSGuard -->|Read / Write| Workspace[Local Project Folder]
```

### Migration Steps
1.  **Extract Rust Backend**: Re-package the current Rust backend commands into a standalone execution binary (sidecar). Expose commands (path validation, Ollama checks, SQLite logging) via a local WebSocket or JSON-RPC loop.
2.  **Generate Branded Theia App**: Create a branded Theia application using the `@theia/cli`. Customize the layout to contain the custom Agent timeline panel.
3.  **Integrate Visual System**: Configure custom CSS themes within Theia to apply Bahamut's Soft Black, Muted Olive, and Dusty Rose palette, including accessibility settings.
