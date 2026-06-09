# Bahamut Security Model

Bahamut places safety and user control first. It operates under a strict security perimeter where the local filesystem and terminal are fully audited.

## Core Security Mechanisms

### 1. Project Directory Sandbox
- **Strict Canonical Path Validation**: Every read, write, or query operation canonicalizes the target path and verifies that it starts with the canonical path of the user's active project.
- **Symbolic Link Protections**: Target paths that resolve to symbolic links pointing outside the project folder are explicitly rejected.
- **Default Ignores**: Bahamut automatically filters and ignores dangerous or large folders: `.git`, `node_modules`, `target`, build artifacts, binaries, and oversized files.

### 2. Context Protection & File Mod Safeguards
- **Pre-apply Verification**: Before applying any AI-generated code change, the backend verifies that the target file hasn't been modified on disk since the diff proposal was created. This is done by comparing file hashes.
- **Conflict Handling**: If a file has been modified in the background, Bahamut halts the change, alerts the user of a conflict, and refuses to modify the file until the user regenerates or resolves the conflict.
- **Size & Context Limits**: Enforces configurable file size thresholds and maximum prompt context budgets to prevent memory exhausting or runaway API usage.

### 3. Command Approval & Control
- **Zero Auto-Execution**: No commands run without visual confirmation.
- **Parameters Control**: All commands support timeouts and cancellation tokens. Spawning is handled via discrete child processes.

### 4. Audit Log Database
All interactions are recorded in an audit database (`%APPDATA%/Bahamut/bahamut.db`) which is write-only from the frontend perspective:
- **Proposed Actions**: LLM prompt generation, proposed edits, and suggested commands.
- **Approvals & Rejections**: Logs user choice for commands and code changes.
- **File Modifications**: Success/failure and hash checks of written files.
- **Command Outcomes**: Commands executed, stdout/stderr status, and exit codes.
- **Affected Paths**: System absolute paths involved in operations.
- **Operational Metrics**: Timestamps, cancellation, and timeout events.
