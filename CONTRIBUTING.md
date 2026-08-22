# Contributing to Wino

Thank you for your interest in contributing to **Wino**! We welcome bug fixes, performance improvements, documentation updates, and new optimization rules.

To preserve Wino's core commitment to stability, safety, and transparency, all contributions must adhere to the engineering guidelines outlined below.

---

## 1. Core Contribution Principles

All pull requests and rule additions must satisfy these fundamental criteria:

1. **Zero Placebo**: We do not accept fake RAM flushes, unverified registry hacks, or harmful timer resolution tweaks. All optimizations must provide measurable, documented system benefits.
2. **Strict Reversibility**: Every optimization rule must specify an exact rollback value (`restore_data`) and be verified for 1-click reversibility.
3. **Appropriate Risk Classification**: Every rule must be assigned a verified `RiskLevel`:
   - `Safe`: Non-destructive, zero risk of feature breakage, fully reversible without reboot.
   - `Low`: Modifies optional consumer features (e.g. Widgets feed, Copilot panel).
   - `Medium`: Modifies diagnostic data collectors or removes OEM promotional stubs. Requires user confirmation.
   - `High`: Modifies advanced network or task configurations. Restricted to manual per-item execution.
   - `Critical`: Core kernel components (`RpcSs`, `WinDefend`, `wuauserv`, `DcomLaunch`). **Hard-blocked from modification.**
4. **Zero PowerShell Spawning**: All system operations must interact directly with native Win32 APIs, Registry trees, or the Service Control Manager. Spawning `powershell.exe` for scanning or queries is strictly prohibited.
5. **Protection of Critical Subsystems**: Essential Windows kernel services, identity subsystems, and security providers must never be modified or disabled.

---

## 2. Declarative Rule System

Optimization rules are declaratively defined as structured JSON files within the `data/` directory:

- `data/debloat_rules.json`: AppX package removals, promotional feature toggles, and UI debloating.
- `data/service_rules.json`: Windows service startup configurations and safety classifications.
- `data/privacy_rules.json`: Telemetry, diagnostics, and privacy policy toggles.
- `data/cleanup_rules.json`: Storage cleaner targets, directory paths, and file extensions.

### Example Rule Specification (`data/debloat_rules.json`)

```json
{
  "id": "disable_example_feature",
  "name": "Disable Example Background Telemetry Collector",
  "description": "Prevents Windows from running unnecessary background sampling routines.",
  "category": "features",
  "preset": "Safe",
  "risk": "Safe",
  "reversible": true,
  "requires_admin": true,
  "supported_windows": ["10", "11"],
  "reason": "Reduces background CPU wakeups and minimizes idle disk I/O.",
  "estimated_benefit": "Reduces idle memory footprint by ~50 MB and silences telemetry logging.",
  "registry_keys": [
    {
      "hive": "HKLM",
      "path": "Software\\Policies\\Microsoft\\Windows\\Example",
      "value_name": "DisableExampleFeature",
      "value_type": "DWORD",
      "value_data": 1,
      "restore_data": 0
    }
  ],
  "package_names": [],
  "services": []
}
```

---

## 3. Development & Verification Workflow

### Prerequisites
* Rust 1.75+ with the GNU (`x86_64-pc-windows-gnu`) or MSVC (`x86_64-pc-windows-msvc`) toolchain.
* Windows 10 (Build 19041+) or Windows 11.

### Validation Checklist
Before submitting a pull request, run the following automated verification suite:

```powershell
# 1. Format check
cargo fmt --check

# 2. Static analysis and linting
cargo clippy --all-targets --all-features

# 3. Execute automated test suite (16 unit & integration tests)
cargo test

# 4. Verify release compilation
cargo build --release
```

---

## 4. Submitting a Pull Request

1. **Fork the repository** on GitHub and create a feature branch from `main`:
   ```powershell
   git checkout -b feature/my-optimization-rule
   ```
2. **Commit your changes** with a concise, descriptive commit message following Conventional Commits:
   ```powershell
   git commit -m "feat(debloat): add rule to disable Windows 11 Widgets news feed"
   ```
3. **Push to your fork** and open a Pull Request against `main`.
4. Provide a clear description of the proposed change, including technical rationale, performance benefits, and verification steps.

---

*Thank you for helping build a faster, safer, and cleaner Windows experience with Wino!*
