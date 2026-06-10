# Bahamut Dependency & Licensing Inventory

This document tracks the license status and redistribution obligations of
dependencies used in the Bahamut desktop application
(`apps/bahamut-desktop/`). The Eclipse Theia / Electron rows were removed
when the Theia spike was retired (ADR-002).

---

## 1. Desktop Shell Framework

| Dependency | License | Type | Notes & Redistribution Obligations |
| :--- | :--- | :--- | :--- |
| **Tauri v2** (incl. `tauri-build`, `@tauri-apps/api`, `@tauri-apps/cli`) | MIT / Apache-2.0 | Permissive | Redistributable; requires copyright notice inclusion. |
| **tauri-plugin-dialog** | MIT / Apache-2.0 | Permissive | Native folder/file picker only. |

---

## 2. Editor

| Dependency | License | Type | Status / Review |
| :--- | :--- | :--- | :--- |
| **Monaco Editor** (`monaco-editor` npm) | MIT | Permissive | Bundled locally by Vite (no CDN). Approved. |

*Warning*: do not bundle extensions from the Microsoft Visual Studio
Marketplace; its terms restrict use to Microsoft products. (Bahamut bundles
no VS Code extensions.)

---

## 3. NPM Dependencies

| Dependency | License | Type | Status / Review |
| :--- | :--- | :--- | :--- |
| **React / React DOM** | MIT | Permissive | Approved. |
| **TypeScript** | Apache-2.0 | Permissive | Approved. |
| **Vite / @vitejs/plugin-react** | MIT | Permissive | Build tooling (dev dependency). Approved. |
| **Vitest** | MIT | Permissive | Test tooling (dev dependency). Approved. |

---

## 4. Rust Backend Crates

| Crate | License | Type | Status / Review |
| :--- | :--- | :--- | :--- |
| **tauri** | MIT / Apache-2.0 | Permissive | Approved. |
| **rusqlite** (bundled SQLite) | MIT | Permissive | Approved. SQLite itself is public domain. |
| **sha2** | MIT / Apache-2.0 | Permissive | Approved. |
| **sysinfo** | MIT | Permissive | Approved. |
| **reqwest** | MIT / Apache-2.0 | Permissive | Approved. |
| **serde / serde_json** | MIT / Apache-2.0 | Permissive | Approved. |
| **tokio** | MIT | Permissive | Approved. |

---

## 5. Licensing Flags & Audit

- **Copyleft (GPL / AGPL / SSPL)**: **none** in Bahamut's core codebase or
  runtime dependencies.
- **Non-Commercial / Source-Available**: **none**.
- **Redistribution Strategy**: all major components (Tauri, Monaco, React,
  Rust crates) are under permissive licenses (MIT, Apache-2.0), so Bahamut
  can be fully repackaged, branded, and distributed without license
  conflicts.
