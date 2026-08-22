use clap::{Parser, Subcommand};
use crate::cleaner::cleaner::execute_cleanup;
use crate::cleaner::scanner::scan_cleaner_targets;
use crate::context_menu::scanner::scan_context_menu_handlers;
use crate::core::system::SystemInfo;
use crate::debloat::executor::apply_debloat_preset;
use crate::debloat::scanner::scan_debloat_items;
use crate::gaming::optimizer::enable_gaming_profile;
use crate::health::diagnostics::evaluate_system_health;
use crate::memory::monitor::capture_memory_snapshot;
use crate::memory::optimizer::optimize_memory;
use crate::network::dns::{find_preset, list_adapters, set_adapter_dns, ALL_PRESETS};
use crate::network::flush::flush_dns_cache;
use crate::privacy::scanner::scan_privacy_items;
use crate::restore::rollback::rollback_snapshot;
use crate::restore::snapshots::list_snapshots;
use crate::services::scanner::scan_services;
use crate::startup::scanner::scan_startup_items;
use crate::tasks::manager::set_task_enabled;
use crate::tasks::scanner::scan_scheduled_tasks;

#[derive(Parser, Debug)]
#[command(name = "wino")]
#[command(author = "Wino Development Team")]
#[command(version = "2.5.0")]
#[command(about = "Rust-Native Windows Debloater, Optimizer & Memory Suite", long_about = None)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Scan overall system status and optimization opportunities
    Scan,
    /// Evaluate Windows security, update, and disk health
    Health,
    /// Memory monitoring and intelligent working set optimization
    Memory {
        #[command(subcommand)]
        action: Option<MemoryCommands>,
    },
    /// Windows debloating and package management
    Debloat {
        #[command(subcommand)]
        action: Option<DebloatCommands>,
    },
    /// Inspect and manage Windows startup applications
    Startup,
    /// Inspect Windows services and safety ratings
    Services,
    /// Privacy and telemetry settings inspection
    Privacy,
    /// Clean temporary files, caches, and logs
    Cleanup {
        #[command(subcommand)]
        action: Option<CleanupCommands>,
    },
    /// Activate Windows Game Mode and background memory optimization
    Gaming {
        #[arg(long)]
        dry_run: bool,
    },
    /// Restore configuration snapshots and undo modifications
    Restore {
        #[command(subcommand)]
        action: Option<RestoreCommands>,
    },
    /// Context menu shell handler management
    #[command(name = "contextmenu")]
    ContextMenu {
        #[command(subcommand)]
        action: Option<ContextMenuCommands>,
    },
    /// DNS flush and preset-based DNS switching (native Win32, zero PowerShell)
    Dns {
        #[command(subcommand)]
        action: Option<DnsCommands>,
    },
    /// Inspect and toggle telemetry scheduled tasks (native schtasks)
    Tasks {
        #[command(subcommand)]
        action: Option<TasksCommands>,
    },
}

#[derive(Subcommand, Debug)]
pub enum MemoryCommands {
    /// Display real-time memory metrics and calculated pressure
    Status,
    /// Execute working set trimming and memory optimization
    Optimize {
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum DebloatCommands {
    /// Scan installed packages and eligible debloat items
    Scan,
    /// Apply debloat preset (Safe, Balanced, Aggressive)
    Apply {
        #[arg(short, long, default_value = "Safe")]
        preset: String,
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum CleanupCommands {
    /// Scan temporary files and calculate reclaimable space
    Scan,
    /// Remove safe temporary files and caches
    Apply {
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum RestoreCommands {
    /// List all saved local configuration snapshots
    List,
    /// Rollback system configuration to a specific snapshot ID
    Apply {
        snapshot_id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum ContextMenuCommands {
    /// List all Explorer context menu handlers
    Scan,
    /// Toggle a handler (enable or disable by handler key name)
    Toggle {
        handler_name: String,
        #[arg(long)]
        enable: bool,
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum DnsCommands {
    /// List available DNS presets and current adapter configuration
    Scan,
    /// Flush the resolver cache via native DnsFlushResolverCache
    Flush,
    /// Apply a DNS preset to all adapters (auto / cloudflare / google / quad9)
    Apply {
        #[arg(short, long)]
        preset: String,
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum TasksCommands {
    /// List detected telemetry and updater scheduled tasks
    Scan,
    /// Enable or disable a task by full path (see tasks scan output for paths)
    Toggle {
        task_path: String,
        #[arg(long)]
        enable: bool,
        #[arg(long)]
        dry_run: bool,
    },
}

pub fn run_cli(command: Commands) {
    match command {
        Commands::Scan => {
            println!("================================================================================");
            println!("                           WINO SYSTEM STATUS SCAN                              ");
            println!("================================================================================");
            let sys = SystemInfo::detect();
            let mem = capture_memory_snapshot();
            let health = evaluate_system_health();

            println!("OS:             {} {} (Build {})", sys.os_name, sys.display_version, sys.build_number);
            println!("Architecture:   {}", sys.architecture);
            println!("Privileges:     {}", if sys.is_admin { "Administrator (Elevated)" } else { "Standard User" });
            println!("Overall Health: {:?}", health.rating);
            println!("RAM Usage:      {:.1}% ({:.2} GB / {:.2} GB)", mem.stats.usage_pct, mem.stats.used_bytes as f64 / 1e9, mem.stats.total_bytes as f64 / 1e9);
            println!("Memory Pressure: {}", mem.pressure.as_str());
            println!("\nRecommendations:");
            for r in &health.recommendations {
                println!("  - {}", r);
            }
        }
        Commands::Health => {
            let health = evaluate_system_health();
            println!("System Health: {:?}", health.rating);
            if !health.issues.is_empty() {
                println!("\nIssues Detected:");
                for i in &health.issues {
                    println!("  [!] {}", i);
                }
            }
            println!("\nActionable Recommendations:");
            for r in &health.recommendations {
                println!("  [*] {}", r);
            }
        }
        Commands::Memory { action } => {
            match action.unwrap_or(MemoryCommands::Status) {
                MemoryCommands::Status => {
                    let mem = capture_memory_snapshot();
                    println!("================================================================================");
                    println!("                             WINO MEMORY ENGINE                                 ");
                    println!("================================================================================");
                    println!("Physical RAM:       {:.2} GB", mem.stats.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0));
                    println!("Used Memory:        {:.2} GB ({:.1}%)", mem.stats.used_bytes as f64 / (1024.0 * 1024.0 * 1024.0), mem.stats.usage_pct);
                    println!("Available Memory:   {:.2} GB", mem.stats.available_bytes as f64 / (1024.0 * 1024.0 * 1024.0));
                    println!("System Cache:       {:.2} MB", mem.stats.cached_bytes as f64 / (1024.0 * 1024.0));
                    println!("Commit Charge:      {:.2} GB / {:.2} GB", mem.stats.commit_used_bytes as f64 / (1024.0 * 1024.0 * 1024.0), mem.stats.commit_limit_bytes as f64 / (1024.0 * 1024.0 * 1024.0));
                    println!("Memory Pressure:    {}", mem.pressure.as_str());
                    println!("Active Processes:   {}", mem.stats.process_count);
                }
                MemoryCommands::Optimize { dry_run } => {
                    let rep = optimize_memory(dry_run);
                    println!("{}", rep.message);
                }
            }
        }
        Commands::Debloat { action } => {
            match action.unwrap_or(DebloatCommands::Scan) {
                DebloatCommands::Scan => {
                    let items = scan_debloat_items();
                    println!("Found {} debloat targets:", items.len());
                    for it in &items {
                        println!("  - [{}] {} (Preset: {}, Risk: {:?}) - Status: {}", it.rule.category, it.rule.name, it.rule.preset, it.rule.risk, it.status_text);
                    }
                }
                DebloatCommands::Apply { preset, dry_run } => {
                    let results = apply_debloat_preset(&preset, dry_run);
                    println!("Applied preset '{}' ({} actions performed):", preset, results.len());
                    for r in &results {
                        println!("  {} [{}] {}", if r.success { "[OK]" } else { "[FAIL]" }, if r.dry_run { "DRY-RUN" } else { "APPLY" }, r.action);
                    }
                }
            }
        }
        Commands::Startup => {
            let items = scan_startup_items();
            println!("Detected {} startup entries:", items.len());
            for it in &items {
                println!("  - {} (Impact: {}, Source: {}, Publisher: {})", it.name, it.impact, it.source, it.publisher);
            }
        }
        Commands::Services => {
            let items = scan_services();
            println!("Enumerated {} Windows services (showing recommended optimizations):", items.len());
            for it in items.iter().filter(|s| s.classification == "Safe to change" || s.classification == "Optional").take(25) {
                println!("  - {} ({}) - Status: {}, Class: {}, Rec: {}", it.service_name, it.display_name, it.status, it.classification, it.recommended_startup);
            }
        }
        Commands::Privacy => {
            let items = scan_privacy_items();
            println!("Privacy Settings Status ({} policies):", items.len());
            for it in &items {
                println!("  - {} -> {}", it.rule.name, if it.is_applied { "[PROTECTED]" } else { "[DEFAULT]" });
            }
        }
        Commands::Cleanup { action } => {
            match action.unwrap_or(CleanupCommands::Scan) {
                CleanupCommands::Scan => {
                    let items = scan_cleaner_targets();
                    let total_bytes: u64 = items.iter().map(|i| i.total_bytes).sum();
                    println!("Found {:.2} MB recoverable across {} categories:", total_bytes as f64 / (1024.0 * 1024.0), items.len());
                    for it in &items {
                        println!("  - {}: {:.2} MB ({} files)", it.rule.name, it.total_bytes as f64 / (1024.0 * 1024.0), it.file_count);
                    }
                }
                CleanupCommands::Apply { dry_run } => {
                    let items = scan_cleaner_targets();
                    let rep = execute_cleanup(&items, dry_run);
                    println!("{}", rep.message);
                }
            }
        }
        Commands::Gaming { dry_run } => {
            let rep = enable_gaming_profile(dry_run);
            println!("{}", rep.message);
        }
        Commands::Restore { action } => {
            match action.unwrap_or(RestoreCommands::List) {
                RestoreCommands::List => {
                    let snaps = list_snapshots();
                    println!("Available Snapshots ({} total):", snaps.len());
                    for s in &snaps {
                        println!("  - [{}] {} - {}", s.id, s.timestamp, s.description);
                    }
                }
                RestoreCommands::Apply { snapshot_id } => {
                    match rollback_snapshot(&snapshot_id) {
                        Ok(msg) => println!("[OK] {}", msg),
                        Err(err) => eprintln!("[ERROR] {}", err),
                    }
                }
            }
        }
        Commands::ContextMenu { action } => {
            match action.unwrap_or(ContextMenuCommands::Scan) {
                ContextMenuCommands::Scan => {
                    let handlers = scan_context_menu_handlers();
                    println!("Found {} Explorer context menu handlers:", handlers.len());
                    for h in &handlers {
                        println!(
                            "  - {} [{}] CLSID {} DLL='{}' store={:?} — {}",
                            h.friendly_name,
                            h.handler_name,
                            h.clsid,
                            h.dll_path,
                            h.store,
                            if h.is_enabled { "Active" } else { "Disabled" }
                        );
                    }
                }
                ContextMenuCommands::Toggle { handler_name, enable, dry_run } => {
                    let handlers = scan_context_menu_handlers();
                    if let Some(entry) = handlers.iter().find(|h| h.handler_name.eq_ignore_ascii_case(&handler_name)) {
                        match crate::context_menu::manager::toggle_handler(entry, enable, dry_run) {
                            Ok(()) => println!("[OK] Handler '{}' {}.", entry.friendly_name, if enable { "enabled" } else { "disabled" }),
                            Err(e) => eprintln!("[ERROR] {}", e),
                        }
                    } else {
                        eprintln!("Handler '{}' not found. Run `wino contextmenu scan` to list available handlers.", handler_name);
                    }
                }
            }
        }
        Commands::Dns { action } => {
            match action.unwrap_or(DnsCommands::Scan) {
                DnsCommands::Scan => {
                    println!("Available DNS presets:");
                    for p in ALL_PRESETS {
                        let s1 = p.primary.unwrap_or("DHCP");
                        let s2 = p.secondary.unwrap_or("-");
                        println!("  - {:<12} {} , {}", p.id, s1, s2);
                    }
                    let adapters = list_adapters();
                    println!("\nDetected {} adapter(s):", adapters.len());
                    for a in &adapters {
                        println!("  - {} ({}) -> {}", a.friendly_name, a.guid, if a.current_name_server.is_empty() { "Automatic (DHCP)".to_string() } else { a.current_name_server.clone() });
                    }
                }
                DnsCommands::Flush => match flush_dns_cache() {
                    Ok(msg) => println!("[OK] {}", msg),
                    Err(e) => eprintln!("[ERROR] {}", e),
                },
                DnsCommands::Apply { preset, dry_run } => {
                    let Some(p) = find_preset(&preset) else {
                        eprintln!("Unknown preset '{}'. Available: auto, cloudflare, google, quad9", preset);
                        return;
                    };
                    if preset == "auto" {
                        println!("Switching to Automatic (DHCP) DNS...");
                    }
                    let adapters = list_adapters();
                    if adapters.is_empty() {
                        eprintln!("No configurable adapters found.");
                        return;
                    }
                    let mut ok = 0usize;
                    for a in &adapters {
                        println!("  Applying '{}' to {}... dry_run={}", p.name, a.friendly_name, dry_run);
                        if set_adapter_dns(a, p, dry_run).is_ok() {
                            ok += 1;
                        }
                    }
                    if !dry_run && flush_dns_cache().is_ok() {
                        println!("DNS cache flushed.");
                    }
                    println!("[OK] Applied '{}' to {}/{} adapters.", p.name, ok, adapters.len());
                }
            }
        }
        Commands::Tasks { action } => {
            match action.unwrap_or(TasksCommands::Scan) {
                TasksCommands::Scan => {
                    let items = scan_scheduled_tasks();
                    println!("Found {} telemetry/updater tasks:", items.len());
                    for it in &items {
                        println!(
                            "  - {} [{}] — {} ({})  task_path='{}'",
                            it.name,
                            it.category_label,
                            it.description,
                            if it.is_enabled { "Enabled" } else { "Disabled" },
                            it.task_path
                        );
                    }
                }
                TasksCommands::Toggle { task_path, enable, dry_run } => {
                    match set_task_enabled(&task_path, enable, dry_run) {
                        Ok(()) => println!("[OK] Task '{}' {}.", task_path, if enable { "enabled" } else { "disabled" }),
                        Err(e) => eprintln!("[ERROR] {}", e),
                    }
                }
            }
        }
    }
}
