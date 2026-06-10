# Bahamut Dependency & Licensing Inventory

This document tracks the license status and redistribution obligations of dependencies used in the Bahamut desktop application.

---

## 1. Desktop Shell Framework

| Dependency | License | Type | Notes & Redistribution Obligations |
| :--- | :--- | :--- | :--- |
| **Eclipse Theia** | EPL-2.0 | Permissive | Highly customizable. We can rebrand and distribute Bahamut binaries. EPL-2.0 source files that are modified must be made available under EPL-2.0. Secondary licensing options are available for bundling. |
| **Electron** | MIT | Permissive | Standard desktop shell. Redistributable without restrictions, requires inclusion of copyright notices. |

---

## 2. Bundled Editor Extensions

For the Eclipse Theia editor integration, Bahamut packages open-source VS Code extensions from the **Open-VSX registry**:

| Extension / Tool | License | Type | Status / Review |
| :--- | :--- | :--- | :--- |
| **Open-VSX Core Client** | EPL-2.0 | Permissive | Fully compatible. |
| **Monaco Editor (Bundled)** | MIT | Permissive | Built-in web editor module. Permissive. |

*Warning*: Avoid using or bundling extensions from the official Microsoft Visual Studio Marketplace, as Microsoft’s terms of service restrict their use strictly to official Microsoft Visual Studio family products.

---

## 3. NPM Dependencies

| Dependency | License | Type | Status / Review |
| :--- | :--- | :--- | :--- |
| **React** | MIT | Permissive | Approved. |
| **React DOM** | MIT | Permissive | Approved. |
| **TypeScript** | Apache-2.0 | Permissive | Approved. |

---

## 4. Rust Backend Core Crates

| Crate | License | Type | Status / Review |
| :--- | :--- | :--- | :--- |
| **tokio** | MIT | Permissive | Approved. |
| **axum** | MIT | Permissive | Approved. |
| **rusqlite** | MIT | Permissive | Approved. Includes SQLite bundled. Permissive. |
| **sysinfo** | MIT | Permissive | Approved. |
| **reqwest** | MIT / Apache-2.0 | Permissive | Approved. |
| **serde** / **serde_json** | MIT / Apache-2.0 | Permissive | Approved. |

---

## 5. Licensing Flags & Audit

- **Copyleft (GPL / AGPL / SSPL)**: **None** are included in Bahamut’s core codebase or runtime dependencies.
- **Non-Commercial / Source-Available**: **None**.
- **Redistribution Strategy**: Since all major components (Theia, Rust, Electron, Monaco) are under permissive open-source licenses (EPL-2.0, MIT, Apache-2.0), Bahamut can be fully repackaged, branded, and distributed as a closed or open-source product without license conflicts.
