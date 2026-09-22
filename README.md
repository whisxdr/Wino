# Wino: Rust-Native Windows System Management Suite

<div align="center">

[![Rust](https://img.shields.io/badge/Rust-2021_Edition-orange.svg?style=flat&logo=rust)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows_10_%7C_Windows_11-blue.svg?style=flat&logo=windows)](https://microsoft.com/windows)
[![Architecture](https://img.shields.io/badge/Architecture-Native_Win32_%7C_Zero--PowerShell-success.svg?style=flat)](ARCHITECTURE.md)
[![License](https://img.shields.io/badge/License-MIT-green.svg?style=flat)](LICENSE)
[![Tests](https://img.shields.io/badge/Tests-275_Passed-brightgreen.svg?style=flat)]()
[![Release](https://img.shields.io/badge/Version-v2.6.0-blueviolet.svg?style=flat)](https://github.com/whisxdr/Wino/releases)

**A high-performance, transparent, and ultra-low-footprint Windows system management suite built with pure Rust and native Win32 APIs.**

[UI Guide (Screenshots)](docs/UI_GUIDE.md) • [Architecture Guide](ARCHITECTURE.md) • [Contributing](CONTRIBUTING.md) • [Features](#-feature-matrix) • [CLI Reference](#-cli-usage) • [Building](#-building-from-source)

</div>

---

> *"An optimizer should never become a resource hog itself."*

Wino reimagines Windows system management by replacing heavy, brittle PowerShell scripts and opaque tweaks with direct native Win32 kernel calls, strict safety gating, and point-in-time configuration rollback. Inspired by tools such as **Chris Titus Tech WinUtil, Wise Memory Optimizer, Sysinternals Autoruns, and Process Explorer**, Wino delivers instant execution, full transparency, and verifiable results.

**v2.6 adds a full system management surface** on top of the debloat and optimization core: an Application Manager across Win32/Store/AppX/Winget, a composable Profile Engine, a Power Manager, a Windows Features Manager, a Network Center with native diagnostics, a Storage Analyzer, a Health Center, a Security Center, a recommendation engine, and a before/after benchmark. It also fixes a correctness bug: the health dashboard previously reported Windows Update service health without querying the service.

---

## ⚡ Why Wino? Benchmark Comparison

Traditional Windows debloaters spawn multiple synchronous PowerShell subprocesses (`powershell.exe -Command "..."`) to query and modify system state. Each PowerShell invocation consumes 80–150 MB of RAM and requires 1,000–5,000 ms to initialize.

Wino communicates directly with the **Windows Kernel, Win32 APIs, and native Registry trees**:

| Feature / Operation | Legacy PowerShell Optimizers | **Wino (Native Rust / Win32)** | Speedup |
| :--- | :--- | :--- | :--- |
| **AppX Package Scanning** | ~5,000 ms (spawns PowerShell) | **2.6 ms** (`RegEnumKeyExW`) | **~1,900x faster** |
| **Defender & Security Status** | ~3,000 ms (`Get-MpComputerStatus`) | **2.9 ms** (Native Registry query) | **~1,030x faster** |
| **Memory Working Set Trimming** | ~1,000 ms (script loops) | **11.7 ms** (`EmptyWorkingSet`) | **~85x faster** |
| **Service Control Queries** | ~800 ms (`Get-Service` loops) | **1.2 ms** (`OpenSCManagerW`) | **~660x faster** |
| **Idle Memory Footprint** | 120 – 350 MB RAM | **25 – 45 MB RAM** | **~7x lighter** |
| **Console Window Spawning** | Flashes black terminal windows | **Zero console windows** (`#windows_subsystem`) | **Seamless Desktop GUI** |

---

## 🛡️ Core Principles & Safety Architecture

1. **Safety First**: Every operation is evaluated by a strict **Safety & Risk Gating Engine** (`Safe`, `Low`, `Medium`, `High`, `Critical`). Essential kernel components (`RpcSs`, `WinDefend`, `wuauserv`, `DcomLaunch`) are mathematically blocked from being modified.
2. **Zero Placebo**: No fake RAM cleaners, no arbitrary timer resolution hacks, and no dangerous security compromises. Only documented, measurable Windows scheduling and memory optimizations are applied.
3. **Full Transparency**: Every optimization rule clearly displays its unique ID, description, estimated benefit, target registry path, and exact rollback value before execution.
4. **Point-In-Time Snapshots**: Automatic pre-flight snapshot generation before applying any preset or batch modification, enabling 1-click granular rollback.
5. **Real-Time Audit Trail**: All operations, dry-runs, and system events are recorded in an in-memory circular buffer with an interactive live event ticker.

---

## 🚀 Feature Matrix

### 1. ⊞ System Dashboard & Live Telemetry
- Real-time CPU usage histogram, RAM utilization, GPU target, and primary storage telemetry.
- Comprehensive system health status rating (`EXCELLENT`, `GOOD`, `ATTENTION`, `WARNING`, `CRITICAL`).
- Detailed host metadata: Windows 10/11 version, OS build, CPU architecture, and execution privileges.

### 2. ⚡ Memory Engine & Multi-Metric Pressure Analysis
- Intelligent memory pressure assessment analyzing physical RAM, commit charge ratio, available bytes, and cache.
- Non-disruptive working set trimming via `EmptyWorkingSet` targeting idle background processes.
- Detailed memory breakdown: Hardware Reserved, Standby Cache, Available RAM, and Paged Pool.

### 3. ▤ Process Manager & Security Trust Inspector
- Comprehensive active process table displaying PID, memory working set, and digital signature status (`Critical System`, `Signed Trust`, `User App`).
- Instant process termination and working set trimming with critical system process protection.
- Exact-width, non-overlapping tabular layout with real-time search filtering.

### 4. 🧹 Windows Debloat Suite (Safe • Balanced • Aggressive)
- **🛡 Safe Preset (Zero Risk)**: Removes promotional UWP apps (Candy Crush, Solitaire, Tips), disables Bing search in Start Menu, and disables advertising tracking ID. 100% reversible.
- **⚡ Balanced Preset**: Includes Safe plus disables Windows 11 Widgets news feed (saving ~200 MB RAM), Copilot side panel, Edge startup boost, and idle Xbox background services.
- **⚠️ Aggressive Preset**: Deep debloat removing OEM promotional stubs (TikTok, Spotify, Disney) and disabling background location tracking, timeline sync, and feedback surveys.
- Interactive confirmation modal requiring user acknowledgement before applying Balanced or Aggressive presets.

### 5. 🚀 Startup Applications Manager
- Scans `HKCU`/`HKLM` Run and RunOnce keys (both registry views), user and common Startup folders, `StartupApproved` approval records, logon/boot scheduled tasks, and Winlogon entries.
- Reports the **real** enabled state read from the `StartupApproved` records instead of assuming every discovered entry is active.
- Measures boot impact (`High`, `Medium`, `Low`), verifies Authenticode signatures, and shows publisher, full path, and arguments.
- Winlogon and scheduled-task entries are listed read-only; Wino does not toggle sources it cannot safely restore.

### 6. ⚙ Windows Services Manager
- Direct Windows Service Control Manager queries and configuration.
- Clear safety classifications: `Safe to change`, `Usually safe`, `Optional`, `Do not touch`.

### 7. 🛡 Privacy & Telemetry Center
- Granular toggles for Advertising ID, Diagnostic Telemetry levels, Activity History tracking, Inking & Typing personalization, and Location sensors.

### 8. 🗑 Storage Cleaner
- Safe temporary files scanner (`%TEMP%`, `C:\Windows\Temp`, crash dumps, Delivery Optimization cache, shader cache, browser caches) with minimum-age safety filters.

### 9. 🖹 Context Menu Cleaner
- Detects and manages Explorer shell extension handlers across `Classes\Directory` and `Folder`.
- Safely disables sluggish or unneeded third-party context menu handlers with automatic rollback snapshots.

### 10. 🌐 Network Center
- Adapter state from `GetAdaptersAddresses`: IPv4/IPv6 with prefix lengths, gateways, DNS servers, DHCP state, MAC address, link speed, and link state.
- Native diagnostics: flush DNS (`DnsFlushResolverCache`), renew and release DHCP (`IpRenewAddress` / `IpReleaseAddress`), test gateway, ping, DNS lookup, connectivity test, latency, and packet loss, all through ICMP echo with no `ping.exe`.
- DNS presets (Cloudflare, Google, Quad9, AdGuard) are labelled as optional presets, not as a ranking.

### 11. ⏱ Scheduled Tasks Debloater
- Scans Windows Task Scheduler for known telemetry collectors, CEIP tasks, and aggressive third-party auto-updaters.
- Safely disables selected tasks with automated pre-flight XML definition backups.

### 12. 🎮 Gaming Optimization Profile
- One-click configuration for Windows Game Mode, GPU scheduling prioritization, and background latency minimization.

### 13. ✚ System Health Center
- Live state of Windows Update, Microsoft Defender, Windows Firewall, Base Filtering Engine, RPC, pending reboot, and system drive space.
- **Windows Update health is a real service query.** The previous version reported it as healthy without checking; an unqueryable state now reports `Unknown` instead of a fabricated pass.
- Integrated runners for System File Checker (`sfc /scannow`), DISM `CheckHealth`, and DISM `ScanHealth`, executed on worker threads with captured output and no UI freeze.
- The rule throughout: when a check cannot be queried, Wino says `Unknown`.

### 14. ↺ Snapshot & Rollback Engine
- Snapshots now cover seven categories: registry values, services, scheduled tasks, context menu handlers, power settings, network settings, and profile applications.
- Scheduled-task snapshots capture the full XML definition, not just an enabled flag, so a rollback can restore the original triggers and actions.
- Each snapshot reports its operation count, affected categories, and whether a rollback would do anything.
- Restore, Export, Delete, and Inspect per snapshot, with the inspector listing the exact recorded restore set.
- Integration for creating native Windows System Restore Points (VSS).

### 15. 📋 Audit Logs & Live Event Ticker
- Complete chronological record of all optimizations, scans, and system modifications.
- Clickable bottom status pill for instant navigation to audit history.

### 16. 📦 Application Manager *(v2.6)*
- One list across Win32 installed programs, Microsoft Store packages, provisioned AppX packages, and Winget-known packages.
- Per application: display name, publisher, version, install location, install size, install date, package type, source, signature state, and uninstall availability.
- Search, source filter, and sort by name, publisher, size, install date, or version.
- Applications with no removal path show **Uninstall unavailable** rather than a button that cannot work.
- Winget is an optional provider: update checks, single-package updates, and bulk updates work when it is present, and Wino is fully functional when it is not.

### 17. 🎛 Profile Engine *(v2.6)*
- Six built-in profiles (Balanced, Gaming, Performance, Battery Saver, Privacy, Low RAM) plus user-defined profiles.
- Profiles are composed from existing Wino operations, so every step routes through the same Safety Engine, snapshots, and audit log.
- Apply, Preview, Save, Duplicate, Rename, Delete, Export, Import, and Reset.
- The preview shows the exact `OperationDescriptor` set the Safety Engine validates (risk, reversibility, privilege requirement, reboot requirement, current and target state) before anything runs.
- A profile can never bypass the Safety Engine, and a partial application is reported as partial, never as complete success.

### 18. 🔌 Power Manager *(v2.6)*
- Active power plan detection and switching across Balanced, High Performance, Power Saver, and Ultimate Performance, plus any custom scheme.
- Ultimate Performance is not present on a stock install, so creating it is a separate, explicit action from activating it.
- Advanced settings with AC and DC values: processor minimum and maximum state, processor boost behavior, sleep and display timeouts, USB selective suspend, PCI Express link state, and disk idle timeout.
- Every change is snapshotted with the previous AC/DC values, validated by the Safety Engine, and re-read to confirm it took effect. A setting the active scheme does not expose renders no control at all.

### 19. 🧩 Windows Features Manager *(v2.6)*
- Optional Windows components with their state (`Enabled`, `Disabled`, `Requires Reboot`, `Unknown`) and dependencies.
- Curated risk and impact notes for components that materially change system behavior: Hyper-V, Virtual Machine Platform, Windows Sandbox, OpenSSH Server, IIS, Telnet, PowerShell 2.0, and others.
- Dependencies that would be enabled implicitly are named before you confirm.
- Features Wino cannot toggle render as text, never as a dead button.

### 20. 🔒 Security Center *(v2.6)*
- Protection state across Microsoft Defender, Windows Firewall, Secure Boot, TPM (with version), UAC, SmartScreen, Windows Update, and critical security services.
- A disabled protection is reported as a warning, and unqueryable state is reported as `Unknown`.
- **Wino exposes no mechanism to disable a security protection as an optimization.** It reports state and nothing else.

### 21. 💾 Storage Analyzer *(v2.6)*
- Where the space on the system drive actually went: usage by bucket (Applications, Windows, Users, ProgramData, Temp, Other), largest directories, large files, old files, and cache locations.
- Cancellable, budgeted traversal with a depth ceiling, reparse-point skipping, and inaccessible-folder accounting.
- **The analyzer deletes nothing.** Reclaiming space still goes through the Storage Cleaner and its safety workflow.
- Partial scans say so: a limit stop, a cancellation, and skipped paths are all disclosed rather than presented as a complete picture.

### 22. 💡 Recommendations *(v2.6)*
- Measured observations across RAM, commit charge, startup impact, optional services, reclaimable temporary data, power plan, privacy state, security state, update state, disk space, and pending packages.
- Every recommendation carries the measured value it is based on, and every one opens its own view for review.
- **Nothing is applied automatically.** An unqueryable check produces no recommendation at all, because absence of evidence is not evidence of a problem.

### 23. 📊 Before / After Benchmark *(v2.6)*
- Capture measured system state (idle RAM, process count, startup entries, temporary data, power plan, selected service states, selected privacy states), then compare any two captures.
- The comparison reports measured deltas only. There is no estimated-performance field, because any such number would be derived rather than measured.

### 24. 🌐 Dual-Language Support (English / Bahasa Indonesia)
- Full bilingual localization across every navigation tab, panel, header, button, dialog, and toast, including all v2.6 views.
- A parity test asserts every key exists in both languages.

---

## 💻 CLI Usage

Wino features a dual interface. Launch without arguments for the native GUI, or pass subcommands for headless automation:

```powershell
# Run a quick system health scan
wino scan

# Check real-time memory metrics and pressure
wino memory status

# Run a memory optimization dry-run (simulation)
wino memory optimize --dry-run

# Execute memory optimization
wino memory optimize

# Scan debloat targets
wino debloat scan

# Apply debloat presets with dry-run test
wino debloat apply --preset Safe --dry-run
wino debloat apply --preset Balanced

# Scan and clean temporary storage
wino cleanup scan
wino cleanup apply

# List startup applications and impact ratings
wino startup

# Manage configuration restore snapshots
wino restore list
wino restore apply <snapshot_id>
```

### v2.6 commands

```powershell
# Applications: list, check for updates, update, remove
wino apps list
wino apps list --json
wino apps updates
wino apps update --id Mozilla.Firefox
wino apps update --all
wino apps uninstall "Contoso Editor" --dry-run

# Profiles: list, inspect, apply, export, import
wino profile list
wino profile show gaming
wino profile apply gaming --dry-run
wino profile export gaming ./gaming.toml
wino profile import ./gaming.toml

# Power: plans, advanced settings, activate, create Ultimate Performance
wino power plans
wino power settings
wino power set high_performance
wino power create-ultimate

# Windows optional features
wino features list
wino features list --json
wino features set OpenSSH.Client --enable
wino features set TelnetClient --disable --dry-run

# Network center: status and native diagnostics
wino network status
wino network status --json
wino network gateway
wino network ping 1.1.1.1 --count 6
wino network lookup example.com
wino network latency 1.1.1.1
wino network loss 1.1.1.1

# Health center
wino health-center scan
wino health-center scan --json --dism
wino health-center sfc
wino health-center dism-check
wino health-center dism-scan

# Storage analyzer (informational; nothing is deleted)
wino storage scan
wino storage scan --json
wino storage scan --summary
wino storage large-files --limit 50

# Security center
wino security-center scan
wino security-center scan --json

# Measured recommendations
wino recommend
```

Commands that accept `--json` emit a single JSON document on stdout for piping
into another tool. Exit codes are meaningful: a partial profile application, an
unreachable host, a failed DNS lookup, and a critical health verdict all exit
non-zero so a caller can branch on the result instead of parsing prose.

---

## 🛠 Building from Source

### Prerequisites
* Windows 10 (Build 19041+) or Windows 11
* Rust 1.75+ with GNU (`x86_64-pc-windows-gnu`) or MSVC (`x86_64-pc-windows-msvc`) toolchain

```powershell
# Clone the repository
git clone https://github.com/whisxdr/Wino.git
cd Wino

# Verify build
cargo check

# Run automated test suite (275 unit & integration tests)
cargo test

# Static analysis at the project's release gate
cargo clippy --all-targets --all-features -- -D warnings

# Compile optimized release binary
cargo build --release
```

The compiled standalone executable will be located at:
```
target/release/wino.exe
```

---

## 🤝 Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on adding optimization rules, service safety definitions, and coding conventions.

---

## 📄 License

Distributed under the [MIT License](LICENSE). Copyright (c) 2026 whisxdr.
