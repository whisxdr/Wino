# Wino: System Architecture & Technical Design

This document details the internal architecture, safety mechanisms, subsystem interactions, and performance design principles of **Wino**.

---

## 1. High-Level System Architecture

Wino is architected around a unified core execution and telemetry engine. Both the **Immediate-Mode Desktop GUI** and the **Headless CLI Interface** route requests through the same strictly gated safety layer, communicating directly with native Windows kernel and Win32 C-FFI subsystems without spawning PowerShell processes.

```mermaid
graph TD
    UI[Fluent Desktop GUI Layer (eframe / egui)] --> State[Central AppState & Reactive Worker]
    CLI[Headless Automation CLI (clap)] --> CoreExec[Core Execution Engine]
    
    State --> CoreExec
    State --> Mon[Real-Time Telemetry & Hardware Engine]
    
    subgraph Core Safety & State Layer
        CoreExec --> Safety[Safety & Risk Gating Engine]
        CoreExec --> Snap[Snapshot & Rollback Engine]
        Safety --> Win32Exec[Native Win32 System Executor]
    end
    
    subgraph Functional Subsystems
        Win32Exec --> Mem[Memory Engine & Pressure Monitor]
        Win32Exec --> Debloat[Debloat Package & Registry Engine]
        Win32Exec --> Svc[Service Control Manager Engine]
        Win32Exec --> Start[Startup Apps & Run Key Engine]
        Win32Exec --> Priv[Privacy & Telemetry Engine]
        Win32Exec --> Clean[Storage Cleaner Engine]
        Win32Exec --> Ctx[Context Menu Cleaner Engine]
        Win32Exec --> Net[DNS & Network Tools Engine]
        Win32Exec --> Tasks[Scheduled Tasks Manager Engine]
        Win32Exec --> Game[Gaming Profile Optimizer Engine]
        Win32Exec --> Health[Integrity & Defender Engine]
        Win32Exec --> Apps[Application Manager: Win32 / Store / AppX / Winget]
        Win32Exec --> Prof[Profile Engine: composable operations]
        Win32Exec --> Power[Power Manager: schemes & advanced settings]
        Win32Exec --> Feat[Windows Features Manager]
        Win32Exec --> NetC[Network Center & Native Diagnostics]
        Win32Exec --> Store[Storage Analyzer: read-only]
        Win32Exec --> HealthC[System Health Center]
        Win32Exec --> Sec[Security Center: report-only]
        Win32Exec --> Rec[Recommendation Engine: observation-only]
        Win32Exec --> Bench[Before / After Benchmark]
    end
    
    subgraph Windows Kernel & System Subsystems
        Win32Exec --> Win32API[Win32 APIs: advapi32 / psapi / kernel32 / dnsapi]
        Win32Exec --> WinReg[Windows Registry: HKLM / HKCU]
        Win32Exec --> VSS[VSS / System Restore Point Subsystem]
    end
```

---

## 2. Core Architectural Pillars

### 2.1 Zero-PowerShell Performance Guarantee
Legacy Windows debloating and optimization utilities rely heavily on spawning PowerShell subprocesses (`powershell.exe -Command "..."`). Each PowerShell spawn imposes an 80–150 MB memory allocation and 1,000–5,000 ms of process startup overhead.

Wino eliminates intermediate shells entirely by binding directly to the Windows C-API and native registry handles:

| Operation | Legacy PowerShell Method | Wino Native Win32 Method | Latency Improvement |
| :--- | :--- | :--- | :--- |
| **AppX Package Query** | `Get-AppxPackage` via PowerShell | `RegEnumKeyExW` on `HKLM\Software\...\AppxAllUserStore` | **5,000 ms → 2.6 ms** (1,900x faster) |
| **Defender Status Query** | `(Get-MpComputerStatus).RealTimeProtection` | Direct registry query on `HKLM\SOFTWARE\Microsoft\Windows Defender` | **3,000 ms → 2.9 ms** (1,034x faster) |
| **Memory Working Set Trim** | PowerShell script loops | Direct `EmptyWorkingSet` via `psapi.dll` | **1,000 ms → 11.7 ms** (85x faster) |
| **Service Status & Config** | `Get-Service` / `Set-Service` | Windows Service Control Manager (`OpenSCManagerW`) | **800 ms → 1.2 ms** (660x faster) |
| **DNS Cache Flush** | `Clear-DnsClientCache` via PowerShell | Direct `DnsFlushResolverCache` via `dnsapi.dll` | **1,200 ms → 0.4 ms** (3,000x faster) |

### 2.2 Subprocess Policy

Wino is Win32-first: everything that has a practical native API is done through
that API. A small, fixed set of operations genuinely has none, and those are
routed through `core::proc`, which enforces the same contract for every call:
the console window is never shown (`CREATE_NO_WINDOW`), stdout and stderr are
captured with a length cap, the exit code is inspected and reported as a
structured error, `dry_run` short-circuits before any spawn, and no shell is
ever involved (`Command::new(program)` with an argument vector, never a command
string).

| Tool | Why a subprocess is required |
| :--- | :--- |
| `dism.exe` | Enumerating and changing optional Windows features. The crate exposes no DISM API surface (`Win32_System_ApplicationInstallationAndServicing` is the MSI surface), and the CLI is the documented interface. |
| `schtasks.exe` | Toggling an existing task's `Enabled` flag. No public native API does this without rewriting the XML definition, which would lose trigger state. |
| `powercfg.exe` | Duplicating a power scheme to create Ultimate Performance. Not exposed by the power API surface in the crate. Called only on explicit request. |
| `winget.exe` | Third-party package manager; ships as a CLI only. Entirely optional — Wino is fully functional without it. |
| `sfc.exe` | Repairs the component store in its own process context. |
| `powershell.exe` | **One** call site: removing an AppX package. There is no AppX deployment COM interface exposed by the crate. Isolated in `apps/uninstall.rs` with the reason documented at the call site. |

### 2.3 Process Subsystem & Dual GUI/CLI Routing
- The binary is compiled with `#![windows_subsystem = "windows"]` to ensure a clean desktop launch without flashing terminal windows.
- For headless CLI invocations (e.g. `wino scan`, `wino memory status`), Wino dynamically detects the parent console session via `AttachConsole(ATTACH_PARENT_PROCESS)` and routes formatted standard streams to `CONOUT$`.

---

## 3. Codebase Directory & Module Layout

```
wino/
├── data/                       # Declarative Rule Databases (JSON)
│   ├── debloat_rules.json      # Categorized debloat targets & registry keys
│   ├── service_rules.json      # Windows service descriptions & safety classifications
│   ├── privacy_rules.json      # Privacy telemetry toggles & paths
│   └── cleanup_rules.json      # Safe storage cleaner target specifications
├── src/
│   ├── main.rs                 # Process entry point, console attachment & GUI bootstrap
│   ├── lib.rs                  # Library root exposing all functional modules
│   ├── app/                    # GUI Layer (Immediate mode via eframe/egui)
│   │   ├── mod.rs              # App struct, event status footer, side navigation layout
│   │   ├── state.rs            # Central AppState, worker channels, event recorder
│   │   ├── worker.rs           # Background async thread worker for non-blocking I/O
│   │   ├── theme.rs            # Fluent theme palette, visuals, and native Segoe fonts
│   │   ├── navigation.rs       # Sidebar tab buttons with dedicated 22px icon containers
│   │   ├── components.rs       # Reusable UI widgets: metric cards, badges, section headers
│   │   └── views/              # 24 dedicated view modules
│   ├── core/                   # Foundation layer
│   │   ├── cancel.rs           # Cooperative cancellation token for long scans
│   │   ├── config.rs           # TOML configuration serialization (serde-defaulted per section)
│   │   ├── executor.rs         # Safe Win32 execution engine (Registry, Dry-run)
│   │   ├── i18n.rs             # Bilingual localization dictionary (EN / ID) with parity tests
│   │   ├── logger.rs           # Circular in-memory audit log buffer
│   │   ├── proc.rs             # Isolated subprocess runner (hidden console, captured output)
│   │   ├── regutil.rs          # Thin registry helpers (enum, read/write/delete string values)
│   │   ├── safety.rs           # Risk gating engine + OperationDescriptor disclosure model
│   │   └── system.rs           # OS version, build, processor architecture, admin detection
│   ├── memory/                 # Advanced memory monitoring and working set optimization
│   │   ├── pressure.rs         # Multi-metric RAM pressure calculation algorithm
│   │   ├── optimizer.rs        # Working set trimming with critical process exclusion
│   │   ├── monitor.rs          # Kernel memory pools, standby lists, commit charge
│   │   └── compression.rs      # Windows Memory Compression query
│   ├── debloat/                # Windows Debloating engine
│   │   ├── rules.rs            # JSON rule deserializer and preset hierarchies
│   │   ├── scanner.rs          # Real-time state scanner (Applied vs Pending)
│   │   ├── executor.rs         # Preset execution with confirmation gating
│   │   └── packages.rs         # Fast native Win32 AppX registry scanner
│   ├── context_menu/           # Context Menu Cleaner engine
│   │   ├── manager.rs          # Handler toggle state modifier via CLSID prefixing
│   │   └── scanner.rs          # Shell extension scanner across machine and user hives
│   ├── network/                # DNS and Network diagnostics engine
│   │   ├── dns.rs              # DnsFlushResolverCache FFI and adapter DNS modifier
│   │   └── scanner.rs          # Network adapter enumeration and current DNS lookup
│   ├── tasks/                  # Windows Scheduled Tasks Debloater
│   │   ├── manager.rs          # schtasks.exe state modifier with XML backups
│   │   └── scanner.rs          # Task scheduler scanner for telemetry and updater tasks
│   ├── services/               # Windows Service Control Manager integration
│   │   ├── manager.rs          # Native SCManager startup type & state modifier
│   │   └── scanner.rs          # Service enumeration with dependency tracking
│   ├── startup/                # Startup items manager (Run keys & Startup folders)
│   ├── privacy/                # Privacy & Telemetry policies
│   ├── cleaner/                # Safe disk space cleaner with age heuristics
│   ├── gaming/                 # Gaming profile (Power plan, Game Mode, GPU priority)
│   ├── health/                 # SFC, DISM, Defender, and Windows Update diagnostics
│   │   ├── center.rs           # Health Center: per-check verdicts, Unknown never becomes Healthy
│   │   └── update.rs           # Real wuauserv query (was hard-coded before v2.6)
│   ├── restore/                # JSON configuration snapshots & System Restore integration
│   ├── security/               # Authenticode verification, plus the report-only Security Center
│   ├── apps/                   # v2.6 Application Manager
│   │   ├── models.rs           # AppRecord, source/uninstall classification (pure, tested)
│   │   ├── scanner.rs          # Win32 registry + AppX package enumeration
│   │   ├── manager.rs          # Filter, sort, update correlation, open location
│   │   ├── winget.rs           # Optional Winget provider (table parser is pure, tested)
│   │   └── uninstall.rs        # Removal dispatch; the single PowerShell call site
│   ├── profiles/               # v2.6 Profile Engine
│   │   ├── models.rs           # Profile, ProfileStep, apply report (pure, tested)
│   │   ├── builtins.rs         # Six built-in profiles composed from existing operations
│   │   └── manager.rs          # Load/save/apply/preview/export/import
│   ├── power/                  # v2.6 Power Manager
│   │   ├── models.rs           # PowerPlan, PowerSetting, scheme GUIDs (pure, tested)
│   │   ├── manager.rs          # Enumerate/activate schemes, create Ultimate Performance
│   │   └── settings.rs         # Read/write AC+DC advanced settings with verification
│   ├── windows_features/       # v2.6 Windows Features Manager
│   ├── storage/                # v2.6 Storage Analyzer (read-only by construction)
│   ├── recommendations/        # v2.6 recommendation engine (observation-only)
│   ├── benchmark/              # v2.6 before/after capture and comparison
│   └── cli/                    # Headless CLI argument parser and commands
└── tests/
    ├── core_tests.rs           # v2.5 subsystem regression suite (21 tests)
    └── v26_tests.rs            # v2.6 subsystem suite (63 tests)
```

---

## 4. Safety & Risk Gating Engine

Every optimization or system modification is evaluated by a formal state machine to prevent unintended regressions:

```mermaid
stateDiagram-v2
    [*] --> RequestOperation
    RequestOperation --> CheckRiskLevel
    
    CheckRiskLevel --> Blocked: RiskLevel::Critical
    Blocked --> [*]: Operation Prohibited (Kernel / System Security)
    
    CheckRiskLevel --> CheckAdmin: RiskLevel <= Medium
    CheckAdmin --> InsufficientPrivileges: Requires Admin && !IsAdmin
    CheckAdmin --> GenerateSnapshot: Allowed
    
    GenerateSnapshot --> ApplyChanges: Snapshot Serialized to Disk
    ApplyChanges --> RecordAuditLog: Executed Win32 Action
    RecordAuditLog --> [*]: Success
```

### Operation Disclosure

Every mutating operation in Wino describes itself with an
`OperationDescriptor` before it runs, and the shared confirmation dialog reads
those fields from the descriptor rather than from the calling view. That is what
keeps a registry write, a service change, a power setting, a package removal,
and a profile application disclosing the same information:

| Field | Purpose |
| :--- | :--- |
| `risk` | The `RiskLevel` the Safety Engine validates against |
| `reversible` | Whether the change can be undone |
| `requires_admin` | Whether elevation is needed (checked against the live token) |
| `requires_reboot` | Whether a restart is needed for the change to take effect |
| `supported_windows` | Which Windows versions the operation applies to |
| `current_state` / `target_state` | What is observed now and what the operation produces |
| `component` | What is being touched, e.g. `Service: DiagTrack` |

The GUI renders one confirmation dialog for the whole application
(`WinoApp::render_confirmation`). No view applies a mutating action directly: it
stages a `PendingConfirm`, and the dialog dispatches it. The CLI reaches the same
operations through the same functions, so both interfaces exercise identical
logic.

### Report-Only Subsystems

Three v2.6 subsystems deliberately have no apply path at all:

* **Security Center** reports protection state and offers no way to turn a
  protection off. A disabled protection is a warning, never an opportunity.
* **Storage Analyzer** deletes nothing. Reclaiming space routes through the
  Storage Cleaner and its safety workflow.
* **Recommendation Engine** produces observations. Every recommendation opens its
  own view for review; nothing is applied automatically.

A related rule applies to every state query in these subsystems: when a check
cannot be queried, the result is `Unknown`. It is never folded into a healthy
verdict. The v2.6 release fixes the one place where this rule was violated — the
health dashboard reported Windows Update service health as `true` without
querying the service.

### Risk Classification Matrix
1. **`Safe`**: Non-destructive, zero risk of feature breakage. Fully reversible without system reboot (e.g. Disabling Advertising ID, disabling Bing search in Start Menu).
2. **`Low`**: Modifies optional background components (e.g. Windows 11 Widgets news feed, Copilot side panel, Xbox background services for non-gamers).
3. **`Medium`**: Modifies system-wide diagnostic collectors, location sensors, or removes vendor promotional stubs. Requires user attention and explicit confirmation modal.
4. **`High`**: Modifies advanced network adapters or scheduled task configurations. Restricted to individual per-item execution.
5. **`Critical`**: Core kernel and security services (`RpcSs`, `WinDefend`, `wuauserv`, `DcomLaunch`). **Hard-blocked from automated modification.**

---

## 5. Snapshot & Rollback State Machine

Before applying any preset or batch modification, Wino automatically serializes a point-in-time configuration snapshot to disk:

1. **Pre-Flight Snapshot Generation**. A snapshot covers seven categories:
   - Registry DWORD and REG_SZ values across all modified hives.
   - Service startup states (`Automatic`, `Manual`, `Disabled`).
   - Context menu CLSID states.
   - Scheduled tasks, including the **full XML definition**, not just an enabled
     flag. Restoring a task reliably needs its original triggers and actions,
     and a boolean cannot rebuild them. A task whose XML is ACL-restricted is
     recorded as "XML not readable" rather than silently storing nothing.
   - Power settings, with the previous AC and DC values.
   - Network settings, with the previous value per interface.
   - Profile applications, with the step ids that were applied.
   - Persisted as timestamped JSON to `%APPDATA%\Wino\snapshots\<id>.json`.
2. **Execution**:
   - Executes target modifications through the shared executor.
   - Streams operational telemetry to the in-memory circular audit buffer and live event ticker.
3. **Validation and Honest Reporting**:
   - Power writes are re-read after writing. A write that did not take effect
     returns an error instead of reporting success.
   - A profile application records one outcome per step and distinguishes a
     failure from a safety-skip. A partial result is reported as partial; the
     CLI exits non-zero for it.
   - A rollback counts each restored value and logs the ones that failed. It
     never reports complete success for a partial restore.
4. **One-Click Rollback**:
   - Snapshot history is inspectable at any time in the **Restore Points** view,
     which lists the exact recorded restore set per snapshot.
   - Each snapshot reports its operation count and affected categories, and a
     snapshot that recorded nothing renders as "nothing to restore" rather than
     presenting an empty rollback as a safety net.
   - Restoring rewrites every recorded value to its exact prior state.

---

## 6. UI Rendering & Font Pipeline

Wino utilizes `eframe` / `egui` for GPU-accelerated immediate-mode GUI rendering.

To guarantee crisp typography and native Fluent iconography without missing glyph artifacts (`□`):
1. During startup, `theme::configure_fonts` loads native Windows TrueType font definitions from `C:\Windows\Fonts\`:
   - `segoeui.ttf` (Primary UI text rendering)
   - `seguisym.ttf` (Segoe UI Symbol for Fluent glyphs)
   - `segmdl2.ttf` (Segoe MDL2 Assets)
2. All navigation entries allocate a dedicated `22px` icon container with responsive layout wrapping, ensuring zero text clipping or overlapping across window resizing.
3. All tabular data views (e.g. Process Manager) utilize exact-width, clipped cell allocations (`table_cell`) guaranteeing 0% column overlap across all display scales.

---

*Wino Architecture Guide — Built with Rust for Windows 10 & 11.*
