# Wino: Rust-Native Windows Debloater, Optimizer & Memory Engine

<div align="center">

[![Rust](https://img.shields.io/badge/Rust-2021_Edition-orange.svg?style=flat&logo=rust)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows_10_%7C_Windows_11-blue.svg?style=flat&logo=windows)](https://microsoft.com/windows)
[![Architecture](https://img.shields.io/badge/Architecture-Native_Win32_%7C_Zero--PowerShell-success.svg?style=flat)](ARCHITECTURE.md)
[![License](https://img.shields.io/badge/License-MIT-green.svg?style=flat)](LICENSE)
[![Tests](https://img.shields.io/badge/Tests-16%2F16_Passed-brightgreen.svg?style=flat)]()
[![Release](https://img.shields.io/badge/Version-v2.5.0-blueviolet.svg?style=flat)](https://github.com/whisxdr/Wino/releases)

**A high-performance, transparent, and ultra-low-footprint Windows optimization suite built with pure Rust and native Win32 APIs.**

[UI Guide (Screenshots)](docs/UI_GUIDE.md) • [Architecture Guide](ARCHITECTURE.md) • [Contributing](CONTRIBUTING.md) • [Features](#-feature-matrix) • [CLI Reference](#-cli-usage) • [Building](#-building-from-source)

</div>

---

> *"An optimizer should never become a resource hog itself."*

Wino reimagines Windows system management by replacing heavy, brittle PowerShell scripts and opaque tweaks with direct native Win32 kernel calls, strict safety gating, and point-in-time configuration rollback. Inspired by tools such as **Chris Titus Tech WinUtil, Wise Memory Optimizer, Sysinternals Autoruns, and Process Explorer**, Wino delivers instant execution, full transparency, and verifiable results.

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
- Scans `HKCU`/`HKLM` Run keys and user/system Startup folder shortcuts.
- Measures startup boot impact (`High`, `Medium`, `Low`) and verifies Authenticode digital signatures.

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

### 10. 🌐 DNS Optimizer & Network Tools
- Instant DNS resolver cache flush via native `dnsapi.dll` without spawning subprocesses.
- One-click DNS server switching with verified presets (Cloudflare, Google Public DNS, Quad9, AdGuard).

### 11. ⏱ Scheduled Tasks Debloater
- Scans Windows Task Scheduler for known telemetry collectors, CEIP tasks, and aggressive third-party auto-updaters.
- Safely disables selected tasks with automated pre-flight XML definition backups.

### 12. 🎮 Gaming Optimization Profile
- One-click configuration for Windows Game Mode, GPU scheduling prioritization, and background latency minimization.

### 13. ✚ Windows Health & Diagnostics
- Integrated runners for System File Checker (`sfc /scannow`) and DISM component store repair.
- Real-time monitoring for Windows Defender antivirus protection and Windows Update service health.

### 14. ↺ Snapshot & Rollback Engine
- Point-in-time JSON configuration snapshots with 1-click restore.
- Integration for creating native Windows System Restore Points (VSS).

### 15. 📋 Audit Logs & Live Event Ticker
- Complete chronological record of all optimizations, scans, and system modifications.
- Clickable bottom status pill for instant navigation to audit history.

### 16. 🌐 Dual-Language Support (English / Bahasa Indonesia)
- Full bilingual localization across all navigation tabs, headers, buttons, modal dialogs, and toast notifications.

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

# Run automated test suite (16 unit & integration tests)
cargo test

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
