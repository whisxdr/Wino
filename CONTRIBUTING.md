# Contributing to Wino

You want to make Wino better. Follow these rules so you keep Wino safe, stable, and light.

Wino puts safety first, then stability, then transparency, then low resource use, then speed.

---

## 1. Ground Rules

You submit a PR when you meet these rules:

1. **Zero Placebo**: You skip fake RAM flushes, unverified registry hacks, and timer tweaks. You ship only changes with measured gain.
2. **Reversible**: You define a revert value for each rule. You test the revert.
3. **Risk Tag**: You tag each rule with `RiskLevel`: `Safe`, `Low`, `Medium`, `High`, `Critical`. You pick the right level for the change.
4. **Zero PowerShell**: You call Win32, Registry, or Service Control Manager. You never spawn `powershell.exe` for a scan or query.
5. **Guard Core**: You leave `RpcSs`, `WinDefend`, `wuauserv`, and `DcomLaunch` alone. Wino blocks changes to those.

---

## 2. Add a New Rule

You keep rules as JSON in `data/`:

- `data/debloat_rules.json`: AppX, feature toggles, and promo stubs.
- `data/service_rules.json`: Service info and safety tag.
- `data/privacy_rules.json`: Telemetry and privacy toggles.
- `data/cleanup_rules.json`: Cleaner paths and extensions.

### Example Rule (`data/debloat_rules.json`)

```json
{
  "id": "disable_example_feature",
  "name": "Disable Example Unnecessary Background Task",
  "description": "Explains what this toggle changes in plain English.",
  "category": "features",
  "preset": "Safe",
  "risk": "Safe",
  "reversible": true,
  "requires_admin": true,
  "supported_windows": ["10", "11"],
  "reason": "Why this save matters: you cut background work and disk use.",
  "estimated_benefit": "You save ~50 MB idle RAM and you cut disk churn.",
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

You fill `description` and `reason` in plain words. You name the benefit in MB or seconds when you can.

---

## 3. Dev Workflow

```powershell
# 1. Format and lint
cargo fmt --check
cargo clippy

# 2. Run tests (8 unit and integration tests)
cargo test

# 3. Build release
cargo build --release
```

You run all three before you push.

---

## 4. Open a Pull Request

1. You fork `whisxdr/Wino` and you branch from `main`:
   ```powershell
   git checkout -b feature/my-enhancement
   ```
2. You commit with a clear message:
   ```powershell
   git commit -m "Add rule to turn off Widgets feed"
   ```
3. You push to your fork and you open a PR against `main`. You describe what you changed, why, and how you tested the revert.
