# Wino — Rust-Native Windows Debloater, Optimizer & Memory Management Suite

<div align="center">

[![Rust](https://img.shields.io/badge/Rust-2021_Edition-orange.svg?style=flat&logo=rust)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows_10_%7C_Windows_11-blue.svg?style=flat&logo=windows)](https://microsoft.com/windows)
[![Architecture](https://img.shields.io/badge/Architecture-Native_Win32_%7C_Zero--PowerShell-success.svg?style=flat)](ARCHITECTURE.md)
[![License](https://img.shields.io/badge/License-MIT-green.svg?style=flat)](LICENSE)
[![Tests](https://img.shields.io/badge/Tests-8%2F8_Passed-brightgreen.svg?style=flat)]()

**A modern, transparent, ultra-low-footprint Windows optimizer designed with safety, stability, and zero placebo as primary principles.**

[Architecture Guide](ARCHITECTURE.md) • [Contributing](CONTRIBUTING.md) • [Features](#-feature-matrix) • [CLI Reference](#-cli-usage) • [Building](#-building-from-source)

</div>

---

> **The Defining Principle of Wino:**  
> *"An optimizer should not become a resource hog itself."*

Wino combines the most useful concepts of tools such as **Chris Titus Tech WinUtil, Wise Memory Optimizer, Sysinternals Autoruns, Process Explorer, and modern Windows debloat utilities**, while completely eliminating their heavy PowerShell script execution loops, fragile regex parsing, and risky system modifications.

---

## ⚡ Why Wino? (Benchmark Comparison)

Traditional Windows debloaters rely heavily on slow, synchronous PowerShell subprocesses (`powershell.exe -Command "..."`). Each PowerShell spawn consumes 80–150 MB of RAM and takes 1,000–5,000 ms to initialize.

Wino interacts directly with the **Windows Kernel, Win32 APIs, and native Registry trees**:

| Feature / Operation | Legacy PowerShell Optimizers | **Wino (Native Rust / Win32)** | Speedup |
| :--- | :--- | :--- | :--- |
| **AppX Debloat Scanning** | ~5,000 ms (spawns PowerShell) | **2.6 ms** (`RegEnumKeyExW`) | **~1,900x faster** |
| **Defender & Security Status** | ~3,000 ms (`Get-MpComputerStatus`) | **2.9 ms** (Native Registry query) | **~1,030x faster** |
| **Memory Optimization Cycle** | ~1,000 ms (heavy script loops) | **11.7 ms** (`EmptyWorkingSet`) | **~85x faster** |
| **Service Control Queries** | ~800 ms (`Get-Service` loops) | **1.2 ms** (`OpenSCManagerW`) | **~660x faster** |
| **Idle Memory Footprint** | 120 – 350 MB RAM | **25 – 45 MB RAM** | **~7x lighter** |
| **Console Spawning** | Flashes black terminal windows | **Zero console windows** (`#windows_subsystem`) | **Clean Desktop GUI** |

---

## 🛡️ Core Principles & Safety Architecture

1. **Safety First**: Every single operation is verified by a strict **Safety & Risk Gating Engine** (`Safe`, `Low`, `Medium`, `High`, `Critical`). Critical kernel components (`RpcSs`, `WinDefend`, `wuauserv`) are mathematically blocked from being disabled.
2. **Zero Placebo**: No fake RAM flushes, no arbitrary timer resolution registry hacks, and no dangerous security deactivations.
3. **Full Transparency**: Every rule clearly details its ID, description, estimated benefit, registry hive, and exact revert configuration.
4. **Point-In-Time Restore Snapshots**: Automatic pre-flight snapshot generation before applying any batch optimizations with 1-click granular rollback.
5. **Real-Time Live Event Ticker**: Interactive status pill on the bottom bar tracking all system actions with direct one-click navigation to audit history.

---

## 🚀 Feature Matrix

### 1. ⊞ System Dashboard & Live Telemetry
- Real-time CPU, RAM, GPU (Dedicated VRAM / Shared Memory), Primary Drive (C:), and Network I/O telemetry.
- Overall System Health Rating: `EXCELLENT`, `GOOD`, `ATTENTION`, `WARNING`, `CRITICAL`.
- Host metadata: Windows 10/11 version, Build number, CPU Architecture, and Admin status.

### 2. ⚡ Memory Engine & Pressure Analysis
- Multi-dimensional memory pressure algorithm analyzing physical RAM, commit charge ratio, available memory, and cache.
- Non-disruptive working set trimming (`EmptyWorkingSet`) for inactive background tasks.
- Windows Memory Compression analysis and kernel memory pool tracking.

### 3. 🧹 Windows Debloat Suite (Safe • Balanced • Aggressive)
- **🛡 Safe Preset**: Removes consumer bloatware (Solitaire, Feedback Hub, Tips), disables Bing search in Start Menu, and disables advertising tracking ID. Zero feature breakage.
- **⚡ Balanced Preset**: Includes Safe PLUS disables Windows 11 Widgets news feed (saves ~200MB RAM), Windows Copilot side panel, Microsoft Edge startup boost, and idle Xbox services for non-gamers.
- **⚠️ Aggressive Preset**: Deeply removes OEM promotional stubs (TikTok, Disney, Spotify, Prime Video), disables background location sensors, activity timeline cloud sync, and feedback survey prompts.
- **Attention & Confirmation Modal**: Clear interactive confirmation dialog before applying Balanced or Aggressive presets.

### 4. ▤ Process Inspector
- Enumerates running processes with PID, working set, private commit bytes, thread count, and executable paths.
- Instant process memory trimming and termination with critical process exemptions.

### 5. 🚀 Startup Applications Manager
- Scans `HKCU`/`HKLM` Run keys and Startup folder shortcuts.
- Measures startup impact rating (`Low`, `Medium`, `High`) and verifies Authenticode digital signatures.

### 6. ⚙ Windows Services Manager
- Direct Windows Service Control Manager queries.
- Safety classifications: `Safe to change`, `Usually safe`, `Optional`, `Do not touch`.

### 7. 🛡 Privacy & Telemetry Center
- Granular toggles for Advertising ID, Diagnostic Data levels, Activity History, Inking & Typing dictionaries, and Location sensors.

### 8. 🗑 Storage Cleaner
- Safe temporary files scanner (`%TEMP%`, `C:\Windows\Temp`, crash dumps, Delivery Optimization cache, shader cache, browser cache) with minimum age safety filters.

### 9. 🎮 Gaming Optimization Profile
- Power Plan switching, Windows Game Mode configuration, and pre-game background memory cleanup.

### 10. ✚ Windows Health & Diagnostics
- Integrated SFC (`sfc /scannow`) and DISM component store repair runners.
- Live Defender antivirus and Windows Update status monitors.

### 11. ↺ Snapshot & Rollback Engine
- Point-in-time JSON configuration snapshots with 1-click restore.
- Native Windows System Restore Point creation integration.

### 12. 📋 Audit Logs
- Comprehensive in-memory circular audit buffer recording all operations, dry-runs, and execution timestamps.

---

## 💻 CLI Usage

Wino features a dual interface. Launch without arguments for the native GUI, or pass subcommands for scriptable, headless CLI automation:

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

# Apply debloat preset with dry-run test
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

# Run compiler checks
cargo check

# Run the full automated test suite (8 unit & integration tests)
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

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on adding new debloat rules, service safety definitions, and coding standards.

---

## 📄 License

Licensed under the [MIT License](LICENSE).
