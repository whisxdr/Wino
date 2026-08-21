# Contributing to Wino

Thank you for your interest in improving **Wino**!

Wino is built with an uncompromising focus on **Safety → Stability → Transparency → Low Resource Usage → Performance**.

---

## 1. Ground Rules for Contributions

Before submitting a Pull Request, please ensure your changes adhere to these non-negotiable guidelines:

1. **Zero Placebo**: We do not accept fake RAM flushes, registry hacks with unverified claims, or timer resolution manipulations.
2. **Reversibility**: Every optimization or rule must have a corresponding, exact rollback / restore value defined.
3. **Risk Level Classification**: Every rule must be classified with a precise `RiskLevel` (`Safe`, `Low`, `Medium`, `High`, `Critical`).
4. **Zero-PowerShell Subprocesses**: Never invoke `powershell.exe` for scanning or basic queries. Use direct Win32 APIs, Windows Registry bindings, or Windows Service Control Manager APIs.
5. **No Breaking Changes to Windows Core**: Critical kernel services (`RpcSs`, `WinDefend`, `wuauserv`, `DcomLaunch`) must remain strictly protected.

---

## 2. Adding New Rules

Rules are declaratively defined as JSON files in the `data/` directory:

- `data/debloat_rules.json`: AppX packages, feature toggles, and promotional stubs.
- `data/service_rules.json`: Windows services descriptions, recommendations, and safety ratings.
- `data/privacy_rules.json`: Diagnostic telemetry and privacy policies.
- `data/cleanup_rules.json`: Safe disk cleaner paths and file extensions.

### Example Rule Schema (`data/debloat_rules.json`):
```json
{
  "id": "disable_example_feature",
  "name": "Disable Example Unnecessary Background Task",
  "description": "Explains exactly what this toggle modifies in plain English.",
  "category": "features",
  "preset": "Safe",
  "risk": "Safe",
  "reversible": true,
  "requires_admin": true,
  "supported_windows": ["10", "11"],
  "reason": "Technical rationale explaining why disabling this saves resources.",
  "estimated_benefit": "Saves 50 MB idle RAM and reduces background disk churn.",
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

## 3. Development Workflow

```powershell
# 1. Verify code formatting and linting
cargo fmt --check
cargo clippy

# 2. Run all unit and integration tests
cargo test

# 3. Ensure clean compilation in release mode
cargo build --release
```

---

## 4. Submitting a Pull Request

1. Fork the repository and create your branch from `main`:
   ```powershell
   git checkout -b feature/my-enhancement
   ```
2. Commit your changes with a descriptive commit message:
   ```powershell
   git commit -m "Add telemetry rule for Windows Widgets background indexing"
   ```
3. Push to your fork and submit a Pull Request to `whisxdr/Wino`.
