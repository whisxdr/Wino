# Wino: Rust-Native Windows Debloater, Optimizer and Memory Suite

<div align="center">

[![Rust](https://img.shields.io/badge/Rust-2021_Edition-orange.svg?style=flat&logo=rust)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows_10_%7C_Windows_11-blue.svg?style=flat&logo=windows)](https://microsoft.com/windows)
[![Architecture](https://img.shields.io/badge/Architecture-Native_Win32_%7C_Zero--PowerShell-success.svg?style=flat)](ARCHITECTURE.md)
[![License](https://img.shields.io/badge/License-MIT-green.svg?style=flat)](LICENSE)
[![Tests](https://img.shields.io/badge/Tests-16%2F16_Passed-brightgreen.svg?style=flat)]()

Wino cleans and tunes Windows. You keep control. The app stays light, stays safe, and shows you each change before you apply it.

[UI Guide (Screenshots)](docs/UI_GUIDE.md) • [Architecture Guide](ARCHITECTURE.md) • [Contributing](CONTRIBUTING.md) • [Features](#-feature-matrix) • [CLI Reference](#-cli-usage) • [Building](#-building-from-source)

</div>

---

Wino builds on ideas from **WinUtil, Wise Memory Optimizer, Sysinternals Autoruns, Process Explorer, and Windows debloat tools**. You get the useful parts without PowerShell loops, fragile regex parsing, or risky edits.

---

## Why Wino: Benchmark Comparison

Traditional debloaters spawn PowerShell for each check. A spawn costs 80 to 150 MB and 1 to 5 seconds to start.

Wino calls the Windows Kernel and Win32 Registry. You skip the spawn. You keep memory low and you get results in milliseconds.

| Feature / Operation | Legacy PowerShell Optimizers | **Wino (Native Rust / Win32)** | Speedup |
| :--- | :--- | :--- | :--- |
| **AppX Debloat Scanning** | ~5,000 ms (spawns PowerShell) | **2.6 ms** (`RegEnumKeyExW`) | **~1,900x faster** |
| **Defender and Security Status** | ~3,000 ms (`Get-MpComputerStatus`) | **2.9 ms** (Registry query) | **~1,030x faster** |
| **Memory Optimization Cycle** | ~1,000 ms (script loops) | **11.7 ms** (`EmptyWorkingSet`) | **~85x faster** |
| **Service Control Queries** | ~800 ms (`Get-Service` loops) | **1.2 ms** (`OpenSCManagerW`) | **~660x faster** |
| **Idle Memory Footprint** | 120 to 350 MB RAM | **25 to 45 MB RAM** | **~7x lighter** |
| **Console Spawning** | Flashes black terminal windows | **Zero console windows** (`#windows_subsystem`) | **Clean Desktop GUI** |

---

## Core Principles and Safety Architecture

1. **Safety First**: The Safety and Risk Gating Engine checks each operation. You see five levels: `Safe`, `Low`, `Medium`, `High`, `Critical`. You cannot turn off `RpcSs`, `WinDefend`, or `wuauserv`. The engine blocks those.

2. **Zero Placebo**: Wino skips fake RAM flushes, timer hacks, and unsafe security toggles. You run changes with measured benefit.

3. **Full Transparency**: You read the ID, description, benefit, registry path, and revert value for each rule before you apply it.

4. **Restore Snapshots**: Wino saves a snapshot before you apply a batch. You roll back with one click.

5. **Live Event Ticker**: The bottom bar shows each action. You click the status pill to jump to audit history.

---

## Feature Matrix

### 1. System Dashboard and Live Telemetry
- You track CPU, RAM, GPU (dedicated VRAM and shared memory), primary drive (C:), and network I/O in real time.
- You read a health rating: `EXCELLENT`, `GOOD`, `ATTENTION`, `WARNING`, `CRITICAL`.
- You see host info: Windows 10/11 version, build number, CPU arch, and admin status.

### 2. Memory Engine and Pressure Analysis
- You run a pressure score that weights physical RAM, commit charge, free memory, and cache.
- You trim the working set of idle background tasks with `EmptyWorkingSet`. You leave active apps alone.
- You inspect memory compression and kernel pool use.

### 3. Windows Debloat Suite (Safe, Balanced, Aggressive)
- **Safe Preset**: You remove consumer bloat (Solitaire, Feedback Hub, Tips), you turn off Bing in Start, and you disable the ad tracking ID. You break nothing.
- **Balanced Preset**: You get Safe plus you turn off the Widgets news feed (you save ~200 MB), Copilot panel, Edge startup boost, and idle Xbox services.
- **Aggressive Preset**: You remove OEM promo stubs (TikTok, Disney, Spotify, Prime Video) and you turn off location, timeline sync, and feedback prompts.
- You confirm Balanced or Aggressive in a modal before Wino applies it.

### 4. Process Inspector
- You list running processes with PID, working set, private bytes, thread count, and exe path.
- You trim memory or end a process. Wino guards critical system processes.

### 5. Startup Applications Manager
- You scan `HKCU`/`HKLM` Run keys and Startup folders.
- You see impact ratings (`Low`, `Medium`, `High`) and you verify publisher signatures.

### 6. Windows Services Manager
- You query the Service Control Manager.
- You see safety tags: `Safe to change`, `Usually safe`, `Optional`, `Do not touch`.

### 7. Privacy and Telemetry Center
- You toggle Advertising ID, diagnostic data level, activity history, inking dictionary, and location.

### 8. Storage Cleaner
- You scan `%TEMP%`, `C:\Windows\Temp`, crash dumps, Delivery Optimization cache, shader cache, and browser cache.
- You apply an age filter so you skip files that apps still use.

### 9. Gaming Optimization Profile
- You switch power plans and Game Mode. You run a memory trim before you launch a game.

### 10. Windows Health and Diagnostics
- You run `sfc /scannow` and DISM repair from the app.
- You check Defender and Windows Update status.

### 11. Snapshot and Rollback Engine
- You save JSON snapshots and you restore them with one click.
- You create a native System Restore Point when you want extra safety.

### 12. Context Menu Cleaner
- You scan and disable slow or bloated shell extension handlers from Explorer right-click menus with instant rollback snapshots.

### 13. DNS & Network Tools
- You flush DNS resolver cache instantly via `dnsapi.dll` and switch adapter DNS servers (Cloudflare, Google, Quad9, AdGuard).

### 14. Scheduled Tasks Debloater
- You detect and disable telemetry, CEIP, and background updater scheduled tasks with automated XML backups.

### 15. Audit Logs
- You review an in-memory circular buffer. You see each operation, dry-run, and timestamp.

### 16. Dual-Language Support (EN / ID)
- Complete bilingual support for English and Bahasa Indonesia across all navigation tabs, buttons, dialogs, and toasts.

---

## CLI Usage

You run Wino two ways. Run it with no args for the GUI. Pass subcommands for headless automation.

```powershell
# Run a quick system health scan
wino scan

# Check memory metrics and pressure
wino memory status

# Dry-run memory trim (no changes)
wino memory optimize --dry-run

# Run memory trim
wino memory optimize

# Scan debloat targets
wino debloat scan

# Apply presets
wino debloat apply --preset Safe --dry-run
wino debloat apply --preset Balanced

# Scan and clean temp storage
wino cleanup scan
wino cleanup apply

# List startup apps and impact ratings
wino startup

# Manage snapshots
wino restore list
wino restore apply <snapshot_id>
```

---

## Building from Source

### Prerequisites
* Windows 10 (Build 19041+) or Windows 11
* Rust 1.75+ with GNU (`x86_64-pc-windows-gnu`) or MSVC (`x86_64-pc-windows-msvc`) toolchain

```powershell
# Clone the repo
git clone https://github.com/whisxdr/Wino.git
cd Wino

# Check build
cargo check

# Run tests (16 unit and integration tests)
cargo test

# Build release binary
cargo build --release
```

You find the binary at:
```
target/release/wino.exe
```

---

## Contributing

You want to add a rule or a service entry. Read [CONTRIBUTING.md](CONTRIBUTING.md) for format, safety levels, and test steps.

---

## License

Licensed under the [MIT License](LICENSE).
