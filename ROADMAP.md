# Bahamut Roadmap

This document outlines the phased plan for developing Bahamut, a local-first, open-model agentic development environment.

---

## Phase 1: Foundation & Bootstrapping (Current Phase)
- **Tauri Setup**: Configure Tauri v2 backend with a React + TypeScript frontend.
- **SQLite DB & Security Audit Log**: Initialize SQLite database in application app data directory.
- **Strict File Sandbox**: Build backend path-validation guard ensuring file reads/writes are restricted to the user-selected project folder.
- **First-Run Model Setup Wizard (Shell)**: Build the UI flow and checks for detecting Ollama on the user's system, analyzing hardware configuration, and UI presentation of recommended models.

## Phase 2: Navigation & Monaco Integration
- **File Explorer**: React-based file tree that retrieves folder structure within the sandbox.
- **Code Editor**: Embed Monaco Editor for reading, editing, and saving files within the workspace.
- **Dirty State & Saving**: Handle unsaved modifications, file loading, and save states securely.

## Phase 3: Credential Store & Provider Settings
- **Keychain Access**: Integrate the Rust `keyring` crate to store and fetch third-party credentials.
- **Provider Layer**: Define unified Rust trait/types for LLM providers (Ollama, OpenAI, Anthropic).
- **Settings Screen**: Frontend configuration panels to configure local runtime and add credentials.

## Phase 4: Local Chat & Code Diffs
- **Prompt Building & Workspace Context**: Allow users to attach specific code files from the tree to the chat context.
- **Ollama Client**: Backend interface to execute `/api/chat` calls.
- **Monaco Diff Viewer**: Display agent code proposals side-by-side with original contents. Requires manual approval to overwrite files.

## Phase 5: Executable Terminal Integration
- **Terminal Execution Engine**: Rust-based process runner that streams stdout/stderr back to the frontend using Tauri event emitters.
- **Approval Engine**: Visual prompt showing proposed command text before execution.
- **Security Audit Logger integration**: Commit executed commands and file modifications to the local database.
