# Wino Documentation Hub

Welcome to the **Wino** documentation index. All documentation and architectural specifications are self-contained within this repository for offline reference.

---

## Technical Guides & Specifications

| Document | Description |
| :--- | :--- |
| **[UI & Feature Walkthrough Guide](./UI_GUIDE.md)** | Comprehensive visual tour and operational walkthrough of all 24 panels, featuring embedded native screenshots and detailed capability breakdowns. |
| **[Architecture & Technical Design](../ARCHITECTURE.md)** | In-depth engineering specification detailing native Win32 FFI bindings, zero-PowerShell architecture, safety state machines, and the font pipeline. |
| **[Contributing Guidelines](../CONTRIBUTING.md)** | Contribution standards, declarative JSON rule schemas, risk level classifications, and local verification workflows. |

---

## Documentation Assets & Utilities

- **`docs/screenshots/`**: High-resolution PNG screenshots capturing all 24 navigation tabs rendered from live application builds.
- **`scripts/capture.ps1`**: Automated PowerShell capture harness invoking `capture_screenshots.exe` to synchronously capture all UI tabs after visual modifications.

---

*Wino — Built with Rust for Windows 10 & 11.*
