# Bahamut Architecture

Bahamut is a local-first, packaged desktop-first agentic development environment designed for secure and transparent AI-assisted software development.

## System Design

Bahamut is built as a single desktop application packaged using **Tauri v2**. The application consists of a Rust core backend and a React + TypeScript frontend webview. It is not structured as a client-server architecture; all communication is local IPC.

```mermaid
graph TD
    subgraph Frontend [React + TypeScript Frontend]
        UI[React UI Components]
        State[Zustand State Manager]
        Monaco[Monaco Editor]
    end

    subgraph Tauri [Tauri Desktop Application Backend (Rust)]
        TauriCmd[Tauri Command Router]
        FSGuard[File System Guard]
        CmdRunner[Discrete Process Runner]
        Keychain[OS Credential Store Bridge]
        DB[(SQLite Database)]
    end

    subgraph External [External Runtimes]
        Ollama[Local Ollama API]
        OSShell[Windows Shell]
    end

    UI -->|Invoke Command / Channels| TauriCmd
    TauriCmd -->|Restrict Scope| FSGuard
    TauriCmd -->|Execute with Timeout| CmdRunner
    TauriCmd -->|Audit Logs / Transcripts| DB
    TauriCmd -->|Cloud Credentials| Keychain
    FSGuard -->|Strict Read/Write| Workspace[Local Project Folder]
    CmdRunner -->|Spawn Command / Stream Lines| OSShell
    TauriCmd -->|HTTP Client / Pull Model| Ollama
```

### Component Details

1.  **Desktop Framework**: Tauri v2. Unrestricted filesystem and shell plugins are disabled. All OS interaction is performed through custom Rust command handlers exposing a least-privilege interface to the frontend.
2.  **Streaming & IPC**: Tauri **Channels** are used to stream LLM responses and command `stdout`/`stderr` outputs line by line to the React frontend.
3.  **Command Execution**: Discrete child-process command execution (spawning shells like PowerShell or cmd.exe per command) with configurable timeouts and cancellation tokens rather than a persistent interactive PTY.
4.  **Local Database**: SQLite manages settings, configuration, conversational transcripts, and audit logs. The SQLite database file is kept in the OS-specific application data folder (e.g., `%APPDATA%/Bahamut/bahamut.db`).
5.  **Credential Management**: Native OS Credential Store via `keyring` in Rust, reserved for future third-party cloud-provider credentials (Ollama connections do not require API keys).
6.  **AI Orchestration Layer**:
    - **Local Ollama Integration**: Communicates via HTTP requests (`/api/chat`, `/api/tags`, `/api/pull`) to `http://localhost:11434`.
    - **Model Recommendation Engine**: Evaluates available RAM, CPU cores, and GPU properties (using `sysinfo`) and suggests a Qwen model size (e.g., `Qwen2.5-Coder:7b` vs `Qwen2.5-Coder:1.5b`).

---

## File & Context Protection

- **Default Ignores**: Bahamut automatically ignores `.git`, `node_modules`, `target`, build outputs, binaries, and oversized files.
- **Limits**: The application enforces configurable maximum file-size limits and total prompt context windows.
- **Edit Verification**: Before applying an AI-proposed edit, Bahamut computes a hash of the original file and verifies it has not changed since the proposal was generated. If changed, the application refuses to apply the edit and requests a user-initiated review.
