# Wino — Rust-Native Windows Debloater, Optimizer & Memory Suite

> **"An optimizer should not become a resource hog itself."**

**Wino** is a modern, lightweight, safe, transparent, and native Windows control center written in pure Rust. It combines the most effective and safe concepts of tools such as Chris Titus Tech WinUtil, Wise Memory Optimizer, Sysinternals Autoruns, Process Explorer, Windows Settings, and modern debloat utilities into a unified, high-performance suite.

---

## Core Philosophy

- **Safety First**: Zero risky or destructive defaults. Every change is classified with a clear Risk Rating (`Safe`, `Low`, `Medium`, `High`, `Critical`).
- **Zero Placebo**: No fake RAM flushes, no arbitrary timer resolution hacks, and no dangerous security deactivations.
- **Full Transparency**: Every optimization explains *what it does*, *why it is recommended*, and *how to undo it*.
- **Automatic Configuration Snapshots**: Backs up registry and service states before every single modification with one-click granular rollback.
- **Ultra-Low Resource Footprint**: Idle GUI target 20-60 MB RAM with adaptive event-driven polling.

---

## Feature Matrix

### 1. System Dashboard
- Real-time CPU, RAM, GPU (VRAM), Primary Drive (C:), and Network bandwidth telemetry.
- Overall System Health Rating: `EXCELLENT`, `GOOD`, `ATTENTION`, `WARNING`, `CRITICAL`.
- "No critical system changes required" indicator when system is already in prime health.

### 2. Memory Engine & Multi-Metric Pressure Calculation
- Multi-dimensional memory pressure algorithm analyzing physical RAM, available memory, commit charge ratio, and cache.
- Working set trimming (`EmptyWorkingSet`) for inactive background processes without disruptive paging loops.
- Windows Memory Compression inspection.

### 3. Windows Debloater & Package Manager
- Scans pre-installed promotional apps, telemetry collectors, and background feature bloat.
- Built-in presets:
  - **Safe**: Consumer experience, promotional UWP stubs, advertising ID.
  - **Balanced**: Safe + Xbox background services (for non-gamers), Widgets feed, Copilot side panel.
  - **Aggressive**: Deep package cleanup with explicit user gating.
- Full `--dry-run` simulation mode.

### 4. Startup & Service Managers
- **Startup Manager**: Scans Registry (`HKCU`/`HKLM` Run keys) and Startup folders. Classifies impact (`Low`, `Medium`, `High`) and verifies Authenticode digital signatures.
- **Service Browser**: Native Windows Service Control Manager queries with safety classifications (`Safe to change`, `Usually safe`, `Optional`, `Do not touch`). Critical kernel services (`RpcSs`, `WinDefend`, `wuauserv`) are strictly protected.

### 5. Privacy Center
- Granular toggles for Advertising ID, Diagnostic Telemetry levels, Activity History, Inking & Typing dictionaries, and Location sensors.

### 6. Storage Cleaner
- Safe temporary files scanner (`%TEMP%`, `C:\Windows\Temp`, crash dumps, Delivery Optimization caches, shader caches, browser caches) with minimum age filters.
- Reclaims disk space without touching active in-use files.

### 7. Windows Health Diagnostics
- One-click runner for Windows System File Checker (`sfc /scannow`) and DISM component store health checks.
- Real-time Defender antivirus and Windows Update status monitors.

### 8. Gaming Profile
- Configures native Windows Game Mode scheduling and provides measurable background memory trimming before gaming sessions.

### 9. Restore & Audit System
- JSON-based configuration snapshots with one-click granular rollback.
- Native Windows System Restore Point creation integration.
- Complete in-memory and on-disk audit event log.

---

## CLI Usage

Wino features a dual interface. Run without arguments to launch the Fluent GUI, or pass subcommands for headless automation:

```powershell
# System health overview
wino scan

# Memory metrics and pressure status
wino memory status

# Memory optimization (with dry-run simulation)
wino memory optimize --dry-run
wino memory optimize

# Scan debloat targets
wino debloat scan

# Apply debloat preset safely
wino debloat apply --preset Safe --dry-run
wino debloat apply --preset Safe

# Scan temporary files
wino cleanup scan

# Clean temporary files
wino cleanup apply

# List startup entries
wino startup

# List and rollback restore snapshots
wino restore list
wino restore apply <snapshot_id>
```

---

## Building from Source

### Prerequisites
- Windows 10 (Build 19041+) or Windows 11
- Rust 1.75+ (MSVC or GNU toolchain)

```powershell
# Build debug binary
cargo build

# Run unit and integration tests
cargo test

# Build optimized release binary
cargo build --release
```

The compiled binary will be located at `target/release/wino.exe`.

---

## License

Dual-licensed under MIT or Apache-2.0.
