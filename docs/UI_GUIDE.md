# UI Guide: Wino

You see each menu here. You get a screenshot from `docs/screenshots/` and you learn what you click.

All images live in your repo. You need no external host.

---

## Contents

1. [Dashboard](#1--dashboard)
2. [Memory Engine](#2--memory-engine)
3. [Process Manager](#3--process-manager)
4. [Debloater](#4--windows-debloater)
5. [Startup Apps](#5--startup-applications)
6. [Services](#6--windows-services)
7. [Privacy Center](#7--privacy-center)
8. [Storage Cleaner](#8--storage-cleaner)
9. [Gaming Profile](#9--gaming-profile)
10. [Windows Health](#10--windows-health)
11. [Restore and Snapshots](#11--restore--snapshots)
12. [Audit Logs](#12--audit--logs)
13. [Settings](#13--settings)
14. [Context Menu](#14--context-menu-cleaner)
15. [DNS and Network](#15--dns--network)
16. [Scheduled Tasks](#16--scheduled-tasks)

---

## 1. Dashboard

![System Dashboard](./screenshots/01_dashboard.png)

**What you do here:** You track the health of your Windows install in one view.

- You read CPU load and logical cores.
- You read a health score: `EXCELLENT`, `GOOD`, `ATTENTION`, `WARNING`, `CRITICAL`. You act on the tip.
- You see host info: Windows build, GPU, process count.
- You check the footer: admin status, CPU and RAM load, and live events.

---

## 2. Memory Engine

![Memory Engine](./screenshots/02_memory_engine.png)

**What you do here:** You see how Windows uses RAM. You cut pressure without placebo.

- You read a bar for active RAM, free RAM, and standby cache.
- You read commit charge and the pressure badge: `LOW`, `MODERATE`, `HIGH`, `CRITICAL`.
- You click `Optimize Memory Now` to trim idle sets with `EmptyWorkingSet`.
- You click `Simulate` to estimate reclaim before you change anything.

---

## 3. Process Manager

![Process Manager](./screenshots/03_processes.png)

**What you do here:** You inspect running processes and you verify who signed them.

- You search by name or PID.
- You read PID, working set, and publisher (`Verified Signed` or `Unverified`).
- You end a heavy process or you trim its working set. Wino blocks critical system tasks.

---

## 4. Windows Debloater

![Windows Debloater & Optimizer](./screenshots/04_debloat.png)

**What you do here:** You remove bloat, promos, and trackers. You pick a preset that fits your risk.

- `Safe (6 rules)`: You remove bloat like Solitaire and Tips. You turn off Bing in Start and ad ID. You break nothing.
- `Balanced (11 rules)`: You get Safe plus you turn off Widgets feed, Copilot, Edge boost, and idle Xbox. You save ~200 MB.
- `Aggressive (15 rules)`: You remove OEM stubs (TikTok, Disney, Spotify) and you turn off location and timeline sync.
- You see `Optimized` or `Default` for each rule. You preview with Dry-Run and you confirm before you apply Balanced or Aggressive.

---

## 5. Startup Applications

![Startup Applications](./screenshots/05_startup_apps.png)

**What you do here:** You trim what runs at boot.

- You scan `HKCU\Run`, `HKLM\Run`, and Startup folders.
- You read impact: `High`, `Medium`, `Low`.
- You toggle an app off. You keep the install intact.

---

## 6. Windows Services

![Windows Services](./screenshots/06_services.png)

**What you do here:** You tune background services with a safety net.

- You read safety tags: `Safe to change`, `Optional`, `Do not touch`. Red means core. You leave it alone.
- You set a service to `Manual` or `Disabled`.
- You search across hundreds of services.

---

## 7. Privacy Center

![Privacy Center](./screenshots/07_privacy_center.png)

**What you do here:** You lock down tracking.

- You turn off Ad ID.
- You stop activity history and cloud sync.
- You turn off tailored offers and feedback prompts.
- You keep `Protected` status and you revert when you want.

---

## 8. Storage Cleaner

![Storage Cleaner](./screenshots/08_storage_cleaner.png)

**What you do here:** You reclaim disk without touching personal files.

- You see reclaimable size in MB or GB before you clean.
- You target `Temp`, `Windows Temp`, shader cache, Delivery Optimization cache, crash dumps.
- You rely on age filter. You skip files that apps still hold.

---

## 9. Gaming Profile

![Gaming Profile](./screenshots/09_gaming_profile.png)

**What you do here:** You prep Windows for a game.

- You turn on Game Mode. You give the game thread priority.
- You trim RAM before you launch.
- You skip risky hacks. You avoid timer tweaks that crash.

---

## 10. Windows Health

![Windows Health Diagnostics](./screenshots/10_windows_health.png)

**What you do here:** You check system integrity.

- You read the health rating.
- You run `sfc /scannow` to fix system files.
- You run DISM to repair the component store.

---

## 11. Restore and Snapshots

![Restore & Safety Snapshots](./screenshots/11_restore_points.png)

**What you do here:** You save state before you change it. You undo with one click.

- Wino saves a snapshot before you run a batch.
- You create a snapshot by hand when you want.
- You click `Restore` to write old registry and service values back.

---

## 12. Audit and Event Logs

![Audit & Event Logs](./screenshots/12_audit_logs.png)

**What you do here:** You see what ran, when, and what failed.

- You read color tags: `INFO`, `WARN`, `ERROR`.
- You filter by `System`, `Memory`, `Debloat`, `Cleaner`, and more.
- You clear the buffer when you want a fresh view.

---

## 13. Settings

![Settings & Preferences](./screenshots/13_settings.png)

**What you do here:** You set theme and rights.

- You pick `Dark` or `Light`.
- You see `Standard User` or `Administrator`.
- You click `Restart as Administrator` when you need to edit protected keys.
- You set auto-trim threshold for RAM pressure.

---

## 14. Context Menu Cleaner

![Context Menu Cleaner](./screenshots/14_context_menu.png)

**What you do here:** You trim slow right-click entries.

- You search handlers and you count them.
- You see CLSID, DLL path, and publisher.
- You turn an entry off. You turn it back on when you need it.

---

## 15. DNS and Network

![DNS & Network Optimizer](./screenshots/15_network_dns.png)

**What you do here:** You fix DNS and you inspect adapters.

- You flush cache with `DnsFlushResolverCache`. You spawn no CMD.
- You pick a preset: Cloudflare, Google, Quad9, AdGuard, or DHCP auto.
- You view active adapter, IP, and current DNS.

---

## 16. Scheduled Tasks

![Scheduled Tasks](./screenshots/16_scheduled_tasks.png)

**What you do here:** You stop tasks that waste background time.

- You search by name, path, or desc.
- You read tags: `Safe to disable`, `Optional`, `System essential`.
- You toggle the trigger on or off.

---

## Refresh Screenshots

You change the UI. You run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\capture.ps1
```

You compile the capture tool. You open each tab. You save fresh PNGs to `docs/screenshots/`.
