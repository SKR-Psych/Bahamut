# Bahamut Security Model

Bahamut places safety and user control first. It operates under a strict
security perimeter where the local filesystem and terminal are fully audited.
**Rust is the trusted security and execution boundary**: the webview holds no
filesystem or shell permissions, and all sensitive operations pass through
narrow, audited Tauri commands.

## Core Security Mechanisms

### 1. Project Directory Sandbox
- **Strict Canonical Path Validation**: every read, write, or query
  operation canonicalizes the target path and verifies it resolves inside
  the canonical path of the user's active project. Validation happens **at
  the point of use** on every command — validated paths are never cached
  across user actions (TOCTOU discipline).
- **Symbolic Link Protections**: target paths that resolve through symbolic
  links to locations outside the project folder are rejected.
- **Windows-specific protections**: NTFS alternate-data-stream syntax
  (`file.txt:stream`) and reserved DOS device names (`NUL`, `CON`, `COM1`…,
  with or without extension) are rejected before canonicalization; verbatim
  (`\\?\`), case-folded, and 8.3 short-name spellings are covered by an
  adversarial test suite.
- **Default Ignores**: the project tree filters `.git`, `node_modules`,
  `target`, build outputs, binary files, and oversized files
  (configurable `max_file_size_bytes` setting, default 2 MiB).

### 2. Context Protection & File Modification Safeguards
- **Pre-write Verification** (implemented): `save_project_file` re-reads the
  target file and verifies its SHA-256 hash equals the hash handed out when
  the file was opened. If the file changed on disk in the meantime, the save
  is refused, the conflict is audit-logged, and the user must reload.
- **Pre-change Snapshots & Rollback** (implemented): every save stores the
  previous file content in SQLite before writing. Any snapshot can be
  restored; the restore revalidates the stored path against the *current*
  project root, snapshots the current content first (so the rollback is
  itself reversible), and writes atomically.
- **Atomic Writes** (implemented): file content is written to a temporary
  file in the same directory, fsynced, then renamed over the target — a
  crash cannot leave a half-written file.
- **Binary & Size Limits**: reads reject files containing NUL bytes or
  invalid UTF-8 and files above the configured size limit; saves enforce the
  same limit.

### 3. Webview Lockdown (Tauri configuration)
- **Capabilities**: the main window holds only `core:default` (IPC plumbing)
  and `dialog:allow-open` (native folder picker — user-driven, returns a
  path string only). No fs, shell, or opener plugin permissions.
- **Content Security Policy**: `script-src 'self'` with no remote origins;
  `connect-src` limited to the Tauri IPC endpoint; frames and objects
  disabled. Monaco is bundled locally — the application loads nothing from
  a CDN.
- **Application identity**: `com.samirahman.bahamut`.

### 4. Command Approval & Control (Phase 5)
- **Zero Auto-Execution**: no commands run without visual confirmation.
- **Parameters Control**: all commands will support timeouts and
  cancellation tokens; spawning via discrete child processes with
  environment scrubbing.

### 5. Audit Log Database
All interactions are recorded in an audit database
(`%APPDATA%/Bahamut/bahamut.db`) which is write-only from the frontend
perspective. The log is **tamper-evident via hash chaining; verification
detects any modification or deletion**: every entry stores a monotonically
increasing sequence number and an `entry_hash` computed as SHA-256 over
(sequence number ‖ previous entry hash ‖ canonical JSON serialisation of the
entry payload). The first entry chains from a fixed genesis value, and the
current chain head is mirrored in the settings table so deletion of trailing
rows is also detected. The `verify_audit_chain` command walks the table and
reports the first broken link, allowing the UI to display an "audit log
verified" status. Note this is tamper *evidence*, not tamper *prevention*: a
local attacker with database access can still modify the file, but any
modification or deletion is detectable on verification.

Recorded events:
- **Workspace selection**: `set_project_root` successes.
- **File Modifications**: every `save_file` (success, conflict, or denied)
  with path and previous/new content hashes and the snapshot id.
- **Rollbacks**: every `rollback_file` with snapshot id, restored hash, and
  undo-snapshot id; denied rollback attempts.
- **Denied Access Attempts**: sandbox rejections from read/save/rollback.
- **Future**: proposed LLM actions, approvals/rejections, command outcomes
  (Phases 4–5).

## Local AI and Read-Only Chat Security

The local AI chat milestone is read-only: it can inspect explicitly attached text and files, but it cannot modify files, run terminal commands or launch autonomous agents. Repository text is treated as untrusted and is wrapped with a fixed system boundary stating that project content cannot override Bahamut security rules, permissions or the milestone boundary.

Attachment reads revalidate paths at point of use through the existing sandbox guard, reject traversal and symlink escapes, skip ignored directories, reject binaries, enforce per-file and total-context limits and report truncation. Secret scanning returns categories and locations only; full secret values are not written to logs or audit entries. Files with possible secrets require explicit user confirmation before submission, and only metadata is audited.

Ollama endpoint validation defaults to loopback and rejects arbitrary remote hosts. Cloud API keys remain out of SQLite and plaintext settings.

## Future Observability and Governance Security (planning only)

Phases 12–14 must preserve the secrets boundary, permission model, audit system, retention controls and explicit user consent for telemetry exports. OpenTelemetry, Langfuse, OpenMetadata, Soda Core and Git-related integrations require clean-room and licence review before implementation.
