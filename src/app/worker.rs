//! Background job engine.
//!
//! Heavy scans (disk walks, service enumeration, package registry walks,
//! storage analysis, network diagnostics) and mutating actions (debloat apply,
//! memory trim, file cleanup, package removal, profile apply, power changes)
//! run on OS worker threads. The egui UI thread only sends [`Job`]s and drains
//! [`JobResult`]s each frame, keeping rendering smooth with no micro-stutter.

use crate::apps::models::{AppRecord, WingetUpdate};
use crate::benchmark::models::BenchmarkSample;
use crate::cleaner::cleaner::execute_cleanup;
use crate::cleaner::scanner::{scan_cleaner_targets, ScannedCleanItem};
use crate::context_menu::scanner::{scan_context_menu_handlers, ContextMenuEntry};
use crate::debloat::executor::{apply_debloat_preset, apply_debloat_rule};
use crate::debloat::rules::DebloatRule;
use crate::debloat::scanner::{scan_debloat_items, ScannedDebloatItem};
use crate::health::center::{run_health_scan, HealthReport};
use crate::memory::optimizer::{optimize_memory, OptimizationReport};
use crate::monitoring::process::{list_running_processes, ProcessItem};
use crate::network::diagnostics::{run_network_scan, NetworkReport};
use crate::power::manager::{list_plans, PowerPlan};
use crate::power::settings::PowerSetting;
use crate::privacy::scanner::{scan_privacy_items, ScannedPrivacyItem};
use crate::profiles::models::{Profile, ProfileApplyReport};
use crate::recommendations::models::Recommendation;
use crate::restore::snapshots::{list_snapshots, Snapshot};
use crate::security::center::{scan_security_center, SecurityReport};
use crate::services::scanner::{scan_services, ServiceItem};
use crate::startup::scanner::{scan_startup_items, StartupItem};
use crate::storage::models::StorageTree;
use crate::tasks::scanner::{scan_scheduled_tasks, ScheduledTaskItem};
use crate::windows_features::models::WindowsFeature;
use std::sync::mpsc::{channel, Receiver, Sender};

/// Identifies an asynchronous scan job (Copy + Hash for pending tracking).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScanKind {
    Processes,
    Debloat,
    Startup,
    Services,
    Privacy,
    Cleaner,
    Snapshots,
    ContextMenu,
    Tasks,
    // ---- v2.6 ----
    Apps,
    AppUpdates,
    Profiles,
    Power,
    Features,
    Network,
    Health,
    Security,
    Storage,
    Recommendations,
}

impl ScanKind {
    /// Human label used in status toasts.
    pub fn label(&self) -> &'static str {
        match self {
            ScanKind::Processes => "Processes",
            ScanKind::Debloat => "Debloat",
            ScanKind::Startup => "Startup",
            ScanKind::Services => "Services",
            ScanKind::Privacy => "Privacy",
            ScanKind::Cleaner => "Storage Cleaner",
            ScanKind::Snapshots => "Restore Snapshots",
            ScanKind::ContextMenu => "Context Menus",
            ScanKind::Tasks => "Scheduled Tasks",
            ScanKind::Apps => "Applications",
            ScanKind::AppUpdates => "Application Updates",
            ScanKind::Profiles => "Profiles",
            ScanKind::Power => "Power",
            ScanKind::Features => "Windows Features",
            ScanKind::Network => "Network",
            ScanKind::Health => "System Health",
            ScanKind::Security => "Security Center",
            ScanKind::Storage => "Storage Analyzer",
            ScanKind::Recommendations => "Recommendations",
        }
    }
}

/// A one-shot mutating action executed off-thread.
#[derive(Debug, Clone)]
pub enum Action {
    ApplyDebloatPreset(String),
    ApplyDebloatRule(Box<DebloatRule>),
    OptimizeMemory(bool),
    CleanupFiles(Vec<ScannedCleanItem>),
    // ---- v2.6 ----
    UninstallApp {
        record: Box<AppRecord>,
        dry_run: bool,
    },
    UpdateWingetPackage {
        package_id: String,
        dry_run: bool,
    },
    UpdateAllWinget {
        dry_run: bool,
    },
    ApplyProfile {
        profile: Box<Profile>,
        dry_run: bool,
    },
    SetPowerPlan {
        plan_id: String,
        dry_run: bool,
    },
    SetPowerSetting {
        setting: Box<PowerSetting>,
        ac_value: u32,
        dc_value: u32,
        dry_run: bool,
    },
    ToggleWindowsFeature {
        feature_name: String,
        enable: bool,
        dry_run: bool,
    },
    ToggleStartupEntry {
        item: Box<StartupItem>,
        enable: bool,
        dry_run: bool,
    },
    RunSfc {
        dry_run: bool,
    },
    RunDism {
        scan_health: bool,
        dry_run: bool,
    },
    RollbackSnapshot {
        snapshot_id: String,
    },
    DeleteSnapshot {
        snapshot_id: String,
    },
    ExportSnapshot {
        snapshot_id: String,
        destination: String,
    },
    CaptureBenchmark {
        label: String,
    },
    /// Run one network diagnostic off the UI thread. `tool` names the probe,
    /// `host` is the user-entered target (empty for gateway/connectivity).
    RunNetworkTool {
        tool: NetworkTool,
        host: String,
        count: u32,
    },
}

/// Network diagnostics that are dispatched to a worker.
///
/// These are `Copy` so the action can be cloned into the worker without
/// carrying the report, which the worker re-reads itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkTool {
    TestGateway,
    Connectivity,
    Ping,
    Latency,
    PacketLoss,
    DnsLookup,
}

impl NetworkTool {
    /// Label used in the result header.
    pub fn label(&self) -> &'static str {
        match self {
            NetworkTool::TestGateway => "Test Gateway",
            NetworkTool::Connectivity => "Connectivity Test",
            NetworkTool::Ping => "Ping Host",
            NetworkTool::Latency => "Latency Test",
            NetworkTool::PacketLoss => "Packet Loss",
            NetworkTool::DnsLookup => "DNS Lookup",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Job {
    Scan(ScanKind),
    Run(Box<Action>),
}

#[derive(Debug, Clone)]
pub enum JobResult {
    ScannedProcesses(Vec<ProcessItem>),
    ScannedDebloat(Vec<ScannedDebloatItem>),
    ScannedStartup(Vec<StartupItem>),
    ScannedServices(Vec<ServiceItem>),
    ScannedPrivacy(Vec<ScannedPrivacyItem>),
    ScannedCleaner(Vec<ScannedCleanItem>),
    ScannedSnapshots(Vec<Snapshot>),
    ScannedContextMenu(Vec<ContextMenuEntry>),
    ScannedTasks(Vec<ScheduledTaskItem>),
    // ---- v2.6 ----
    ScannedApps(Vec<AppRecord>),
    ScannedAppUpdates(Vec<WingetUpdate>),
    ScannedProfiles(Vec<Profile>),
    ScannedPower {
        plans: Vec<PowerPlan>,
        settings: Vec<PowerSetting>,
    },
    ScannedFeatures(Vec<WindowsFeature>),
    ScannedNetwork(Box<NetworkReport>),
    ScannedHealth(Box<HealthReport>),
    ScannedSecurity(Box<SecurityReport>),
    ScannedStorage(Box<StorageTree>),
    ScannedRecommendations(Vec<Recommendation>),
    Benchmarked(BenchmarkSample),
    ProfileApplied(ProfileApplyReport),
    /// Result of one network diagnostic, formatted for display.
    NetworkToolDone {
        title: String,
        message: String,
        success: bool,
    },
    MemoryReport(OptimizationReport),
    /// Structured long-running tool output (SFC / DISM). `success` reflects the
    /// tool exit status, `partial` marks an operation that changed some targets
    /// but not all.
    ToolOutput {
        title: String,
        detail: String,
        success: bool,
        partial: bool,
    },
    ActionDone {
        message: String,
        success: bool,
        follow_up: Option<ScanKind>,
    },
    Event {
        category: String,
        message: String,
        is_success: bool,
    },
}

impl JobResult {
    /// The scan kind this result satisfies, if any (used to clear pending flags).
    pub fn scan_kind(&self) -> Option<ScanKind> {
        match self {
            JobResult::ScannedProcesses(_) => Some(ScanKind::Processes),
            JobResult::ScannedDebloat(_) => Some(ScanKind::Debloat),
            JobResult::ScannedStartup(_) => Some(ScanKind::Startup),
            JobResult::ScannedServices(_) => Some(ScanKind::Services),
            JobResult::ScannedPrivacy(_) => Some(ScanKind::Privacy),
            JobResult::ScannedCleaner(_) => Some(ScanKind::Cleaner),
            JobResult::ScannedSnapshots(_) => Some(ScanKind::Snapshots),
            JobResult::ScannedContextMenu(_) => Some(ScanKind::ContextMenu),
            JobResult::ScannedTasks(_) => Some(ScanKind::Tasks),
            JobResult::ScannedApps(_) => Some(ScanKind::Apps),
            JobResult::ScannedAppUpdates(_) => Some(ScanKind::AppUpdates),
            JobResult::ScannedProfiles(_) => Some(ScanKind::Profiles),
            JobResult::ScannedPower { .. } => Some(ScanKind::Power),
            JobResult::ScannedFeatures(_) => Some(ScanKind::Features),
            JobResult::ScannedNetwork(_) => Some(ScanKind::Network),
            JobResult::ScannedHealth(_) => Some(ScanKind::Health),
            JobResult::ScannedSecurity(_) => Some(ScanKind::Security),
            JobResult::ScannedStorage(_) => Some(ScanKind::Storage),
            JobResult::ScannedRecommendations(_) => Some(ScanKind::Recommendations),
            _ => None,
        }
    }
}

/// Execute a job synchronously. Shared by the GUI worker threads and the CLI
/// so both paths exercise identical logic.
pub fn run_job(job: &Job) -> JobResult {
    match job {
        Job::Scan(kind) => match kind {
            ScanKind::Processes => JobResult::ScannedProcesses(list_running_processes()),
            ScanKind::Debloat => JobResult::ScannedDebloat(scan_debloat_items()),
            ScanKind::Startup => JobResult::ScannedStartup(scan_startup_items()),
            ScanKind::Services => JobResult::ScannedServices(scan_services()),
            ScanKind::Privacy => JobResult::ScannedPrivacy(scan_privacy_items()),
            ScanKind::Cleaner => JobResult::ScannedCleaner(scan_cleaner_targets()),
            ScanKind::Snapshots => JobResult::ScannedSnapshots(list_snapshots()),
            ScanKind::ContextMenu => JobResult::ScannedContextMenu(scan_context_menu_handlers()),
            ScanKind::Tasks => JobResult::ScannedTasks(scan_scheduled_tasks()),
            ScanKind::Apps => JobResult::ScannedApps(crate::apps::scanner::scan_installed_apps()),
            ScanKind::AppUpdates => {
                JobResult::ScannedAppUpdates(crate::apps::winget::check_updates())
            }
            ScanKind::Profiles => {
                JobResult::ScannedProfiles(crate::profiles::manager::load_profiles())
            }
            ScanKind::Power => JobResult::ScannedPower {
                plans: list_plans(),
                settings: crate::power::settings::read_all_settings(),
            },
            ScanKind::Features => {
                JobResult::ScannedFeatures(crate::windows_features::scanner::scan_features())
            }
            ScanKind::Network => JobResult::ScannedNetwork(Box::new(run_network_scan())),
            ScanKind::Health => JobResult::ScannedHealth(Box::new(run_health_scan(false))),
            ScanKind::Security => JobResult::ScannedSecurity(Box::new(scan_security_center())),
            ScanKind::Storage => {
                JobResult::ScannedStorage(Box::new(crate::storage::scanner::scan_system_drive()))
            }
            ScanKind::Recommendations => {
                JobResult::ScannedRecommendations(crate::recommendations::analyzer::analyze_system())
            }
        },
        Job::Run(action) => run_action(action),
    }
}

fn run_action(action: &Action) -> JobResult {
    match action {
        Action::ApplyDebloatPreset(preset) => {
            let results = apply_debloat_preset(preset, false);
            JobResult::ActionDone {
                message: format!(
                    "Applied '{}' preset ({} optimizations executed).",
                    preset,
                    results.len()
                ),
                success: true,
                follow_up: Some(ScanKind::Debloat),
            }
        }
        Action::ApplyDebloatRule(rule) => {
            let results = apply_debloat_rule(rule.as_ref(), false);
            JobResult::ActionDone {
                message: format!(
                    "Applied rule '{}' ({} steps executed).",
                    rule.name,
                    results.len()
                ),
                success: true,
                follow_up: Some(ScanKind::Debloat),
            }
        }
        Action::OptimizeMemory(dry_run) => JobResult::MemoryReport(optimize_memory(*dry_run)),
        Action::CleanupFiles(items) => {
            let rep = execute_cleanup(items, false);
            JobResult::ActionDone {
                message: rep.message.clone(),
                success: true,
                follow_up: Some(ScanKind::Cleaner),
            }
        }
        Action::UninstallApp { record, dry_run } => {
            match crate::apps::uninstall::uninstall_app(record, *dry_run) {
                Ok(msg) => JobResult::ActionDone {
                    message: msg,
                    success: true,
                    follow_up: Some(ScanKind::Apps),
                },
                Err(e) => JobResult::ActionDone {
                    message: e,
                    success: false,
                    follow_up: None,
                },
            }
        }
        Action::UpdateWingetPackage {
            package_id,
            dry_run,
        } => match crate::apps::winget::update_package(package_id, *dry_run) {
            Ok(msg) => JobResult::ActionDone {
                message: msg,
                success: true,
                follow_up: Some(ScanKind::AppUpdates),
            },
            Err(e) => JobResult::ActionDone {
                message: e,
                success: false,
                follow_up: None,
            },
        },
        Action::UpdateAllWinget { dry_run } => match crate::apps::winget::update_all(*dry_run) {
            Ok(msg) => JobResult::ActionDone {
                message: msg,
                success: true,
                follow_up: Some(ScanKind::AppUpdates),
            },
            Err(e) => JobResult::ActionDone {
                message: e,
                success: false,
                follow_up: None,
            },
        },
        Action::ApplyProfile { profile, dry_run } => {
            JobResult::ProfileApplied(crate::profiles::manager::apply_profile(profile, *dry_run))
        }
        Action::SetPowerPlan { plan_id, dry_run } => {
            match crate::power::manager::set_active_plan(plan_id, *dry_run) {
                Ok(msg) => JobResult::ActionDone {
                    message: msg,
                    success: true,
                    follow_up: Some(ScanKind::Power),
                },
                Err(e) => JobResult::ActionDone {
                    message: e,
                    success: false,
                    follow_up: None,
                },
            }
        }
        Action::SetPowerSetting {
            setting,
            ac_value,
            dc_value,
            dry_run,
        } => match crate::power::settings::write_setting(setting, *ac_value, *dc_value, *dry_run) {
            Ok(msg) => JobResult::ActionDone {
                message: msg,
                success: true,
                follow_up: Some(ScanKind::Power),
            },
            Err(e) => JobResult::ActionDone {
                message: e,
                success: false,
                follow_up: None,
            },
        },
        Action::ToggleWindowsFeature {
            feature_name,
            enable,
            dry_run,
        } => {
            match crate::windows_features::manager::set_feature_enabled(
                feature_name,
                *enable,
                *dry_run,
            ) {
                Ok(msg) => JobResult::ActionDone {
                    message: msg,
                    success: true,
                    follow_up: Some(ScanKind::Features),
                },
                Err(e) => JobResult::ActionDone {
                    message: e,
                    success: false,
                    follow_up: None,
                },
            }
        }
        Action::ToggleStartupEntry {
            item,
            enable,
            dry_run,
        } => match crate::startup::manager::toggle_startup_item(item, *enable, *dry_run) {
            Ok(()) => JobResult::ActionDone {
                message: format!(
                    "Startup entry '{}' {}.",
                    item.name,
                    if *enable { "enabled" } else { "disabled" }
                ),
                success: true,
                follow_up: Some(ScanKind::Startup),
            },
            Err(e) => JobResult::ActionDone {
                message: e,
                success: false,
                follow_up: None,
            },
        },
        Action::RunSfc { dry_run } => {
            if *dry_run {
                return JobResult::ToolOutput {
                    title: "SFC /scannow".to_string(),
                    detail: "Dry run: no scan started.".to_string(),
                    success: true,
                    partial: false,
                };
            }
            match crate::health::integrity::run_sfc_scan() {
                Ok(out) => JobResult::ToolOutput {
                    title: "SFC /scannow".to_string(),
                    detail: out,
                    success: true,
                    partial: false,
                },
                Err(e) => JobResult::ToolOutput {
                    title: "SFC /scannow".to_string(),
                    detail: e,
                    success: false,
                    partial: false,
                },
            }
        }
        Action::RunDism {
            scan_health,
            dry_run,
        } => {
            let mode = if *scan_health {
                crate::health::integrity::DismMode::ScanHealth
            } else {
                crate::health::integrity::DismMode::CheckHealth
            };
            if *dry_run {
                return JobResult::ToolOutput {
                    title: mode.label().to_string(),
                    detail: "Dry run: no DISM operation started.".to_string(),
                    success: true,
                    partial: false,
                };
            }
            match crate::health::integrity::run_dism(mode) {
                Ok(out) => JobResult::ToolOutput {
                    title: mode.label().to_string(),
                    detail: out,
                    success: true,
                    partial: false,
                },
                Err(e) => JobResult::ToolOutput {
                    title: mode.label().to_string(),
                    detail: e,
                    success: false,
                    partial: false,
                },
            }
        }
        Action::RollbackSnapshot { snapshot_id } => {
            match crate::restore::rollback::rollback_snapshot(snapshot_id) {
                Ok(msg) => JobResult::ActionDone {
                    message: msg,
                    success: true,
                    follow_up: Some(ScanKind::Snapshots),
                },
                Err(e) => JobResult::ActionDone {
                    message: e,
                    success: false,
                    follow_up: None,
                },
            }
        }
        Action::DeleteSnapshot { snapshot_id } => {
            match crate::restore::snapshots::delete_snapshot(snapshot_id) {
                Ok(msg) => JobResult::ActionDone {
                    message: msg,
                    success: true,
                    follow_up: Some(ScanKind::Snapshots),
                },
                Err(e) => JobResult::ActionDone {
                    message: e,
                    success: false,
                    follow_up: None,
                },
            }
        }
        Action::ExportSnapshot {
            snapshot_id,
            destination,
        } => match crate::restore::snapshots::export_snapshot(snapshot_id, destination) {
            Ok(msg) => JobResult::ActionDone {
                message: msg,
                success: true,
                follow_up: None,
            },
            Err(e) => JobResult::ActionDone {
                message: e,
                success: false,
                follow_up: None,
            },
        },
        Action::CaptureBenchmark { label } => {
            JobResult::Benchmarked(crate::benchmark::capture::capture_sample(label))
        }
        Action::RunNetworkTool { tool, host, count } => run_network_tool(*tool, host, *count),
    }
}

/// Execute one network diagnostic on a worker thread.
///
/// The report is re-read here rather than passed in, so the probe uses the
/// adapter state as it is at the moment the diagnostic runs.
fn run_network_tool(tool: NetworkTool, host: &str, count: u32) -> JobResult {
    use crate::network::diagnostics as diag;

    let report = diag::run_network_scan();
    let count = count.max(1);
    let title = tool.label().to_string();

    let outcome: Result<(String, bool), String> = match tool {
        NetworkTool::TestGateway => {
            diag::test_gateway(&report, count).map(|r| (format_ping(&r), r.is_reachable()))
        }
        NetworkTool::Connectivity => {
            diag::test_connectivity(&report, count).map(|r| (format_ping(&r), r.is_reachable()))
        }
        NetworkTool::Ping => {
            if host.is_empty() {
                Err("Enter a host to ping.".to_string())
            } else {
                diag::ping_host(host, count, 1500).map(|r| (format_ping(&r), r.is_reachable()))
            }
        }
        NetworkTool::Latency => {
            if host.is_empty() {
                Err("Enter a host to measure.".to_string())
            } else {
                diag::measure_latency(host, count).map(|r| {
                    let text = match r.avg_rtt_ms() {
                        Some(avg) => format!("{}: {:.1} ms average", r.resolved_address, avg),
                        None => format!("No reply from {}.", r.host),
                    };
                    (text, r.is_reachable())
                })
            }
        }
        NetworkTool::PacketLoss => {
            if host.is_empty() {
                Err("Enter a host to measure.".to_string())
            } else {
                diag::measure_packet_loss(host, count).map(|r| {
                    (
                        format!(
                            "{}: {:.0}% loss ({}/{} replies)",
                            r.host,
                            r.packet_loss_pct(),
                            r.received,
                            r.sent
                        ),
                        r.received == r.sent,
                    )
                })
            }
        }
        NetworkTool::DnsLookup => {
            if host.is_empty() {
                Err("Enter a host to resolve.".to_string())
            } else {
                let r = diag::dns_lookup(host);
                if r.succeeded() {
                    Ok((format!("{}: {}", r.host, r.addresses.join(", ")), true))
                } else {
                    Err(r.summary())
                }
            }
        }
    };

    match outcome {
        Ok((message, success)) => JobResult::NetworkToolDone {
            title,
            message,
            success,
        },
        Err(e) => JobResult::NetworkToolDone {
            title,
            message: e,
            success: false,
        },
    }
}

/// One-line ping summary including the observed spread.
fn format_ping(r: &crate::network::diagnostics::PingResult) -> String {
    let mut text = r.summary();
    if let (Some(min), Some(max)) = (r.min_rtt_ms(), r.max_rtt_ms()) {
        text.push_str(&format!(" min {} ms, max {} ms.", min, max));
    }
    text
}

/// Handles owned by the UI thread to talk with the background engine.
pub struct EngineHandles {
    pub job_tx: Sender<Job>,
    pub job_rx: Receiver<JobResult>,
    /// Auto memory trimmer controls; None when the trimmer failed to spawn.
    pub auto_trim: Option<crate::memory::auto_trim::AutoTrimHandle>,
}

/// Start the dispatcher thread and (optionally) the auto memory trimmer.
pub fn start_engine(auto_trim_enabled: bool) -> EngineHandles {
    let (job_tx, job_rx) = channel::<Job>();
    let (result_tx, job_result_rx) = channel::<JobResult>();

    // Dispatcher thread: pulls jobs and executes them on short-lived workers so
    // long scans never block subsequent requests or the UI frame loop.
    let dispatch_result_tx = result_tx.clone();
    std::thread::Builder::new()
        .name("wino-dispatch".to_string())
        .spawn(move || {
            while let Ok(job) = job_rx.recv() {
                let tx = dispatch_result_tx.clone();
                let _ = std::thread::Builder::new()
                    .name("wino-worker".to_string())
                    .spawn(move || {
                        let _ = tx.send(run_job(&job));
                    });
            }
        })
        .ok();

    let auto_trim = crate::memory::auto_trim::AutoTrimHandle::spawn(result_tx, auto_trim_enabled);

    EngineHandles {
        job_tx,
        job_rx: job_result_rx,
        auto_trim,
    }
}
