# Bahamut Monorepo Workspace

This repository houses the Bahamut desktop-first agentic development environment.

## Workspace Layout

- **`apps/bahamut-desktop/`**: The desktop application shell built on **Eclipse Theia**. Configured with branding, custom layout mode switch (Bahamut IDE vs Bahamut Agent), custom visual themes, and connection bridges to the core service.
- **`services/bahamut-core/`**: The local backend core service built in **Rust**. Manages path-traversal filesystem sandboxing, command execution, and SQLite database audit logging. Acts as a secure sidecar service.
- **`prototypes/tauri-shell/`**: Legacy proof of concept prototype implemented using Tauri v2 and React.
- **`docs/`**: Platform architecture decisions, security models, and product vision specifications.

## Development Requirements

To contribute to this project, your local development environment must meet the following requirements:
- **Node.js**: `20.11.1` (managed via `.nvmrc` files)
- **Yarn**: `1.22.19` (preferred package manager for the Eclipse Theia workspace)
- **Rust**: Stable MSVC toolchain (for the local backend core sidecar service)

