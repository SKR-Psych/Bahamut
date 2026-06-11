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

## Future Expansion Phases (planning only)

The following phases are documented as product direction only. They are not implemented in the current application and must not be represented as available functionality.

### Phase 12 — Native Git and Software Delivery Workflows

Planned native Git and delivery workflows include branch/worktree/stash/tag graph inspection, explicit commit composition, pull-request preparation, CI/CD status review and policy-guarded release/deployment checklists. See `docs/vision/git-workflows.md`.

### Phase 13 — Bahamut Data Intelligence and Governance

Planned data-intelligence work includes metadata connectors, lineage, ownership, glossary, data-quality and data-contract review, governed approvals and retention-aware metadata summaries. See `docs/vision/data-intelligence.md`.

### Phase 14 — Native Data, AI and Agent Observability

Planned observability work includes traces, spans, prompt/model/tool-call event views, redaction, retention controls, agent run replay and interoperability with accepted telemetry formats. See `docs/vision/observability.md`.

### Shared Future Platform Foundation

Phases 12–14 share a future platform foundation: connector SDK, metadata model, graph abstraction, scheduling, event and telemetry bus, policy engine, secrets boundary, permission model, audit system, extension API, agent tool registry, deployment modes, retention controls and interoperability formats.

All future integrations with Git, OpenMetadata, Soda Core, OpenTelemetry and Langfuse require clean-room implementation discipline and licence review before adoption.
