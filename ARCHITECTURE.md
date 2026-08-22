# Wino: System Architecture and Design

You build Wino on a modular, zero-PowerShell stack. Each call passes a safety check before it touches the OS.

---

## 1. High-Level Architecture

You route UI and CLI through the same core engine. You poll telemetry, you gate risk, and you hit Win32. No PowerShell spawns sit in the path.

```mermaid
graph TD
    UI[Fluent GUI Layer (egui / eframe)] --> State[Central AppState & Reactive Poller]
    CLI[Headless CLI Layer (clap)] --> CoreExec[Core Execution Engine]
    
    State --> CoreExec
    State --> Mon[System Telemetry Engine]
    
    subgraph Core Safety & Execution Layer
        CoreExec --> Safety[Safety & Risk Gating Engine]
        CoreExec --> Snap[Snapshot & Rollback Engine]
        Safety --> Win32Exec[Native Win32 System Executor]
    end
    
    subgraph Functional Subsystems
        Win32Exec --> Mem[Memory Engine & Pressure Calculation]
        Win32Exec --> Debloat[Debloat Package & Registry Manager]
        Win32Exec --> Svc[Service Control Manager Engine]
        Win32Exec --> Start[Startup & Run Key Manager]
        Win32Exec --> Priv[Privacy & Telemetry Policies]
        Win32Exec --> Clean[Storage Cleaner Engine]
        Win32Exec --> Game[Gaming Optimization Engine]
        Win32Exec --> Health[Integrity & Defender Engine]
    end

    subgraph OS Kernel & Subsystem
        Win32Exec --> Win32API[Win32 API: advapi32 / psapi / kernel32]
        Win32Exec --> WinReg[Windows Registry: HKLM / HKCU]
        Win32Exec --> VSS[VSS / System Restore Point Subsystem]
    end
```

---

## 2. Core Pillars

### 2.1 Zero-PowerShell Guarantee

Traditional optimizers spawn PowerShell for each query. A spawn like `powershell.exe -Command "Get-AppxPackage"` costs 80 to 150 MB and 500 to 2,000 ms to start. You pay that cost for each toggle.

Wino replaces that spawn with native Win32 C-FFI and Registry calls. You call the API. You get the value. You skip the shell.

| Operation | Legacy PowerShell Method | Wino Native Method | Latency Improvement |
| :--- | :--- | :--- | :--- |
| **AppX Package Query** | `Get-AppxPackage` via PowerShell | `RegEnumKeyExW` on `HKLM\Software\...\AppxAllUserStore` | **5,000 ms to 2.6 ms** (1,900x faster) |
| **Defender Status Query** | `(Get-MpComputerStatus).RealTimeProtection` | Registry query on `HKLM\SOFTWARE\Microsoft\Windows Defender` | **3,000 ms to 2.9 ms** (1,034x faster) |
| **Memory Working Set Trim** | PowerShell script loops | `EmptyWorkingSet` via `psapi.dll` | **1,000 ms to 11.7 ms** (85x faster) |
| **Service Status & Config** | `Get-Service` / `Set-Service` | Service Control Manager (`OpenSCManagerW`) | **800 ms to 1.2 ms** (660x faster) |

### 2.2 Process Subsystem and Console Routing

You compile with `#![windows_subsystem = "windows"]`. You launch the GUI and you see no black console.

For CLI (`wino scan`) you attach to the parent console. You call `AttachConsole(ATTACH_PARENT_PROCESS)` and you write to `CONOUT$`. You reuse the same binary for GUI and CLI.

---

## 3. Module Breakdown

```
d:/Kuliah/Coding/WinOptim/
├── data/                       # Rule databases (JSON)
│   ├── debloat_rules.json      # Debloat targets and registry keys
│   ├── service_rules.json      # Service descriptions and safety tags
│   ├── privacy_rules.json      # Privacy toggles and paths
│   └── cleanup_rules.json      # Cleaner targets
├── src/
│   ├── main.rs                 # Entry point, console attach, GUI bootstrap
│   ├── lib.rs                  # Library root for all modules
│   ├── app/                    # GUI layer (eframe/egui)
│   │   ├── mod.rs              # App struct, status footer, nav layout
│   │   ├── state.rs            # AppState, poller, event recorder
│   │   ├── theme.rs            # Theme palette and Segoe fonts
│   │   ├── navigation.rs       # Sidebar tabs with 22px icon slots
│   │   ├── components.rs       # Metric cards, badges, headers
│   │   └── views/              # 13 view modules (Dashboard, Memory, etc.)
│   ├── core/                   # Foundation
│   │   ├── config.rs           # TOML config
│   │   ├── executor.rs         # Win32 executor, dry-run, Registry
│   │   ├── logger.rs           # Circular audit buffer
│   │   ├── safety.rs           # Risk gate (Safe, Low, Medium, High, Critical)
│   │   └── system.rs           # OS version, build, arch, admin check
│   ├── memory/                 # Memory monitor and trim
│   │   ├── pressure.rs         # RAM pressure score
│   │   ├── optimizer.rs        # Working set trim with guard
│   │   ├── monitor.rs          # Kernel pools, standby, commit charge
│   │   └── compression.rs      # Memory Compression query
│   ├── debloat/                # Debloat engine
│   │   ├── rules.rs            # Rule load and preset hierarchy
│   │   ├── scanner.rs          # State scan (Applied vs Pending)
│   │   ├── executor.rs         # Preset run with confirm gate
│   │   └── packages.rs         # Win32 AppX Registry scan
│   ├── services/               # Service Control Manager
│   │   ├── manager.rs          # Startup type and state control
│   │   └── scanner.rs          # Enum with dependency map
│   ├── startup/                # Run keys and Startup folders
│   ├── privacy/                # Privacy and telemetry
│   ├── cleaner/                # Disk cleaner with age check
│   ├── gaming/                 # Power plan, Nagle, GPU priority
│   ├── health/                 # SFC, DISM, Defender, Update
│   ├── restore/                # JSON snapshots and Restore Point
│   ├── security/               # Authenticode verification
│   └── cli/                    # CLI parser and commands
└── tests/
    └── core_tests.rs           # Unit and integration tests
```

---

## 4. Safety and Risk Gating Engine

You run each action through a risk gate before Wino touches the system.

```mermaid
stateDiagram-v2
    [*] --> RequestOperation
    RequestOperation --> CheckRiskLevel
    
    CheckRiskLevel --> Blocked: RiskLevel::Critical
    Blocked --> [*]: Operation Prohibited (Kernel / System Identity)
    
    CheckRiskLevel --> CheckAdmin: RiskLevel <= Medium
    CheckAdmin --> InsufficientPrivileges: Requires Admin && !IsAdmin
    CheckAdmin --> GenerateSnapshot: Allowed
    
    GenerateSnapshot --> ApplyChanges: Snapshot Created
    ApplyChanges --> RecordAuditLog: Executed Win32 Action
    RecordAuditLog --> [*]: Success
```

### Risk Levels

1. **`Safe`**: You change a reversible toggle. You risk no breakage. You need no restart. Example: you turn off Advertising ID or Bing in Start.

2. **`Low`**: You turn off an optional background feature. Example: you turn off Widgets feed, Copilot panel, or Xbox background tasks.

3. **`Medium`**: You change a diagnostic collector, sensor, or promo stub. You see a confirm dialog before Wino applies it.

4. **`High`**: You change a network or scheduled task. You apply it per item, by hand.

5. **`Critical`**: You touch a kernel component like `RpcSs`, `WinDefend`, `wuauserv`, or `DcomLaunch`. Wino blocks you. You cannot automate this.

---

## 5. Snapshot and Rollback

You change a batch. Wino saves state first so you can undo.

1. **Pre-Flight Snapshot**
   - You capture registry DWORD and string values.
   - You capture service startup types (`Automatic`, `Manual`, `Disabled`).
   - You write timestamped JSON to `%APPDATA%\Wino\snapshots\<id>.json`.

2. **Run**
   - You apply the target values through Win32.
   - You log results to the circular buffer and the event ticker.

3. **Rollback**
   - You open Restore Points (`NavTab::Restore`) and you pick a snapshot.
   - You click restore. Wino writes the old registry and service values back.

---

## 6. UI Rendering and Font Pipeline

You render the GUI with `eframe` and `egui`. You get GPU acceleration and immediate-mode updates.

For crisp Fluent icons you avoid tofu boxes. At startup Wino loads native Windows fonts from `C:\Windows\Fonts\`:

- `segoeui.ttf` (UI text)
- `seguisym.ttf` (Segoe UI Symbol)
- `segmdl2.ttf` (MDL2 Assets)

You allocate a 22px icon slot for each nav item. You wrap text on resize so you never overlap items.
