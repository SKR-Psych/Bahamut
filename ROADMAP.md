# Bahamut Roadmap

This document outlines the phased plan for developing Bahamut, a local-first,
open-model agentic development environment built on Tauri v2 + React +
TypeScript + Monaco (ADR-002).

---

## Phase 1: Foundation & Bootstrapping — ✅ complete
- **Tauri Setup**: Tauri v2 backend with a React + TypeScript frontend.
- **SQLite DB & Security Audit Log**: hash-chained, tamper-evident audit log
  in the application data directory with `verify_audit_chain`.
- **Strict File Sandbox**: canonical path validation guard restricting all
  file access to the user-selected project folder (symlink/ADS/reserved-name
  protections, adversarial test suite).
- **First-Run Model Setup Wizard (Shell)**: Ollama detection, hardware scan,
  model recommendation UI (model download still simulated).
- **Platform consolidation**: Theia spike evaluated and rejected (ADR-002);
  repository consolidated on the Tauri application.

## Phase 2: Navigation & Monaco Integration — ✅ complete (first slice)
- **File Explorer**: filtered project tree (default ignores: `.git`,
  `node_modules`, `target`, build outputs, binaries, oversized files).
- **Code Editor**: bundled Monaco Editor (no CDN) for viewing and editing.
- **Safe Saving**: narrow Rust save command with point-of-use revalidation,
  original-hash verification, pre-change snapshots, atomic writes, and
  snapshot rollback — all audit-logged.

## Phase 3: Credential Store & Provider Settings
- **Keychain Access**: integrate the Rust `keyring` crate to store and fetch
  third-party credentials.
- **Provider Layer**: unified Rust trait/types for LLM providers (Ollama,
  OpenAI, Anthropic).
- **Settings Screen**: configure local runtime, credentials, and the
  configurable file-size limit surfaced in the UI.

## Phase 4: Local Chat & Code Diffs
- **Prompt Building & Workspace Context**: attach files from the tree to the
  chat context.
- **Ollama Client**: backend interface to execute `/api/chat` calls (and a
  real `/api/pull` for the setup wizard).
- **Monaco Diff Viewer**: side-by-side agent proposals with mandatory manual
  approval before any write (reusing the Phase 2 hash-checked save path).

## Phase 5: Executable Terminal Integration
- **Terminal Execution Engine**: Rust process runner streaming stdout/stderr
  to the frontend via Tauri channels.
- **Approval Engine**: visual prompt showing proposed command text before
  execution (zero auto-execution).
- **Audit integration**: executed commands and outcomes recorded in the
  hash-chained audit log; child-process environment scrubbing.
