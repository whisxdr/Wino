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
4. **Native First**: All system operations must interact directly with native Win32 APIs, Registry trees, the Service Control Manager, or the power API. Spawning `powershell.exe` for scanning or queries is prohibited.

   A subprocess is acceptable only where no practical native API exists. There are exactly six such cases in the project, each documented at its call site: `dism.exe` (optional features), `schtasks.exe` (toggling an existing task), `powercfg.exe` (duplicating a power scheme), `winget.exe` (an optional third-party provider), `sfc.exe` (component store repair), and one isolated `powershell.exe` call for AppX package removal. Every call routes through `core::proc`, which hides the console window, captures output with a length cap, inspects the exit code, and honours `dry_run` before spawning. No shell is ever involved: the program is executed with an argument vector, never a command string.

5. **Protection of Critical Subsystems**: Essential Windows kernel services, identity subsystems, and security providers must never be modified or disabled.

6. **Never Report Unknown as Healthy**: When a check cannot be queried, the result is `Unknown`. It is never folded into a passing verdict. This applies to every state query in the health, security, feature, and network subsystems.

7. **No Fabricated Measurements**: A value that was not measured is not displayed. This applies to the recommendation engine, the benchmark, and the power manager (a setting the scheme does not expose renders no control rather than a default of zero).

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
# 1. Format
cargo fmt --all

# 2. Static analysis at the release gate (warnings are errors)
cargo clippy --all-targets --all-features -- -D warnings

# 3. Execute the automated test suite (275 unit & integration tests)
cargo test

# 4. Verify release compilation
cargo build --release
```

### Testing Scope

Tests cover pure decision components: classifiers, thresholds, parsers, and
serialization round-trips. That is deliberate. The decision logic is where a
wrong answer is dangerous, and it is the part that can be verified without a
live Windows subsystem. Behaviours that genuinely need the OS (registry reads,
DISM, ICMP, service control) reach the same pure helpers at runtime and are not
faked in tests.

When adding a subsystem, add a test for its decision logic:

* **Rule databases** (`data/*.json`): every rule must load, and no rule may mark
  critical Windows infrastructure as safe to change.
* **Safety Engine**: Critical is blocked, unsupported Windows is blocked, missing
  administrator privileges are blocked, and safe operations pass.
* **Profiles**: built-ins deserialize and validate, custom profiles round-trip
  through TOML, and a profile containing a Critical step produces a skipped
  outcome without attempting it.
* **Localization**: every key in `KEYS` resolves in both English and Indonesian,
  with no duplicates.

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
