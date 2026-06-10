# Bahamut Architecture

Bahamut is a local-first, packaged desktop-first agentic development
environment designed for secure and transparent AI-assisted software
development. Its production architecture is **Tauri v2 + React + TypeScript +
Monaco Editor** with **Rust as the trusted security and execution boundary**
(ADR-002, Accepted).

## System Design

Bahamut is built as a single desktop application packaged using **Tauri v2**.
The application consists of a Rust core backend and a React + TypeScript
frontend webview. It is not structured as a client-server architecture; all
communication is local IPC. The application lives in `apps/bahamut-desktop/`.

```mermaid
graph TD
    subgraph Frontend [React + TypeScript Frontend]
        UI[React UI Components]
        Tree[Filtered File Tree]
        Monaco[Monaco Editor - bundled, no CDN]
    end

    subgraph Tauri [Tauri Desktop Application Backend (Rust)]
        TauriCmd[Tauri Command Router]
        FSGuard[File System Guard - validate_path]
        Snapshots[(Snapshots)]
        CmdRunner[Discrete Process Runner - Phase 5]
        Keychain[OS Credential Store Bridge - Phase 3]
        DB[(SQLite: audit chain, settings, snapshots)]
    end

    subgraph External [External Runtimes]
        Ollama[Local Ollama API]
        OSShell[Windows Shell - Phase 5]
    end

    UI -->|Invoke Command / Channels| TauriCmd
    TauriCmd -->|Restrict Scope| FSGuard
    TauriCmd -->|Execute with Timeout| CmdRunner
    TauriCmd -->|Audit Logs / Snapshots| DB
    TauriCmd -->|Cloud Credentials| Keychain
    FSGuard -->|Strict Read/Write| Workspace[Local Project Folder]
    CmdRunner -->|Spawn Command / Stream Lines| OSShell
    TauriCmd -->|HTTP Client / Pull Model| Ollama
```

### Component Details

1.  **Desktop Framework**: Tauri v2. Unrestricted filesystem and shell
    plugins are disabled; the only webview capabilities are `core:default`
    and `dialog:allow-open` (native folder picker). A restrictive Content
    Security Policy (`script-src 'self'`, IPC-only `connect-src`) is
    enforced. All OS interaction is performed through custom Rust command
    handlers exposing a least-privilege interface to the frontend.
2.  **Editor**: Monaco Editor bundled locally by Vite (workers included) —
    no CDN dependency, consistent with local-first operation and the CSP.
3.  **File access pipeline** (implemented):
    `list_project_files` → `read_project_file` → `save_project_file` /
    `rollback_file_snapshot`. Every command revalidates the target path at
    the point of use; saves verify the original content hash, store a
    pre-change snapshot, and replace the file atomically.
4.  **Streaming & IPC**: Tauri **Channels** will stream LLM responses and
    command `stdout`/`stderr` line by line to the React frontend (Phase 4/5).
5.  **Command Execution** (Phase 5): discrete child-process execution with
    configurable timeouts and cancellation tokens rather than a persistent
    interactive PTY. Zero auto-execution.
6.  **Local Database**: SQLite in the OS application-data folder
    (`%APPDATA%/Bahamut/bahamut.db`) holding settings, the hash-chained
    audit log, and pre-change file snapshots.
7.  **Credential Management** (Phase 3): native OS credential store via
    `keyring` in Rust, reserved for third-party cloud-provider credentials.
8.  **AI Orchestration Layer**:
    - **Local Ollama Integration**: HTTP requests (`/api/chat`, `/api/tags`,
      `/api/pull`) to `http://localhost:11434`.
    - **Model Recommendation Engine**: evaluates RAM, CPU cores, and GPU
      properties (via `sysinfo`) and suggests a Qwen model size.

---

## File & Context Protection

- **Default Ignores**: the project tree excludes `.git`, `node_modules`,
  `target`, `dist`/`build`/`out`, known binary extensions, and files above
  the configurable size limit (`settings.max_file_size_bytes`, default
  2 MiB). Reads additionally reject NUL bytes and invalid UTF-8.
- **Limits**: maximum file-size limits are enforced on both read and save;
  the tree walk is depth- and entry-capped.
- **Edit Verification** (implemented): before writing, the backend re-reads
  the target and verifies its hash equals the hash handed out when the file
  was opened. On mismatch the save is refused, the conflict is audit-logged,
  and the user must reload and re-apply.
- **Snapshots & Rollback** (implemented): every save stores the previous
  content in SQLite first; any snapshot can be restored atomically, and the
  restore itself snapshots the current content so it can be undone.
