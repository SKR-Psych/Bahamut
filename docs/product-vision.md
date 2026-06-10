# Bahamut Product Vision

Bahamut is a local-first, open-model agentic development environment designed for secure and transparent AI-assisted software development. It targets developers and non-technical users who want the power of modern AI coding agents without sacrificing data ownership or execution safety.

Bahamut integrates two core operational modes into a single, cohesive desktop experience, built on **Tauri v2 + React + TypeScript + Monaco Editor** with **Rust as the trusted security and execution boundary** (ADR-002). Bahamut is local-first, model-neutral, permission-driven, and auditable; all sensitive filesystem and command operations pass through narrow Rust commands.

---

## 1. Bahamut IDE

An interactive, agent-driven Integrated Development Environment (IDE) optimized for real-time collaboration between the user and local AI models.

### Key Capabilities
- **Workspace Navigation**: Explorer tree, multi-tab text and code editing, and global search.
- **Language Support**: Syntax highlighting, auto-complete, diagnostics, and language server integration.
- **Interactive Terminal & Git**: Inline shell control and version control management.
- **AI Assist Panels**:
  - Code explanations, inline refactoring, and auto-completions.
  - Chat interface with access to explicitly attached file contexts.
- **Approval Perimeter**: Interactive, side-by-side Monaco diff viewer where all code modifications must be reviewed and explicitly accepted by the user before writing to disk.

---

## 2. Bahamut Agent

An autonomous, task-oriented agent interface focused on executing larger, complex software engineering objectives.

### Key Capabilities
- **Task Objective Entry**: Input high-level features, bug descriptions, or refactoring plans.
- **Autonomous Execution Loop**:
  - Scans workspace, parses dependencies, and creates a structured implementation plan.
  - Proposes edits to multiple files.
  - Spawns compiler, test suite, or linting commands to verify correctness.
  - Inspects errors and self-corrects code recursively.
- **Audit & Timeline Views**: Displays a live activity timeline showing the agent's current task step, terminal outputs, file diff approvals, and log trails.

---

## Shared Backend Foundation

To ensure consistency, both modes are powered by the same underlying local desktop engine, sharing:
1. **Workspace Context**: The active project folder, ignored path files, index, and cached symbols.
2. **AI Provider Routing**: Model selection, credentials, system hardware profiles, and speed parameters.
3. **Execution & Command Engine**: Discrete process spawning, cancellation tokens, and stdout/stderr stream listeners.
4. **Security & Permission Guard**: Sandboxed path validation rejecting access outside the project root directory.
5. **Unified SQLite Database**: Audit logger (tracking all operations, actions, and user approvals), chat transcripts, and settings.
