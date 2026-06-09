# Bahamut

Bahamut is a local-first, open-model agentic development environment designed for secure and transparent AI-assisted software development. It enables non-technical users to build, edit, run, and understand software projects while keeping absolute control over their local data, files, and execution environment.

## Key Features

- **Strict File Sandboxing**: Restricts file reading and writing strictly to the project directory selected by the user.
- **Zero Auto-Execution**: Safe interactive command runtime. Every command proposed by the AI is displayed in full and requires explicit user approval.
- **Audit Logs**: Maintain a complete, immutable local audit trail of all file changes and terminal actions in a local SQLite database.
- **Model Setup Wizard**: First-run setup that checks if Ollama is installed, scans system hardware (RAM, GPU), suggests the best open-weight coder models (such as Qwen 2.5 Coder), and manages downloads.
- **Monaco Editor Integration**: View and edit project files with side-by-side diff previews for any AI-suggested changes.

## Security Model

For details on how Bahamut protects user files and prevents unauthorized execution, refer to [docs/security.md](docs/security.md).

## Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) (v18+)
- [Rust & Cargo](https://rustup.rs/) (v1.75+)
- [Ollama](https://ollama.com/) (Recommended local runtime)

### Running Bahamut in Development

1. Install dependencies:
   ```bash
   npm install
   ```
2. Start the Tauri development server:
   ```bash
   npm run tauri dev
   ```

## License

This project is licensed under the [MIT License](LICENSE).
