//! Background job engine.
//!
//! Heavy scans (disk walks, service enumeration, package registry walks) and
//! mutating actions (debloat apply, memory trim, file cleanup) run on OS
//! worker threads. The egui UI thread only sends [`Job`]s and drains
//! [`JobResult`]s each frame, keeping rendering at a smooth 60 FPS with no
//! micro-stutter.

use crate::cleaner::cleaner::execute_cleanup;
use crate::cleaner::scanner::{scan_cleaner_targets, ScannedCleanItem};
use crate::context_menu::scanner::{scan_context_menu_handlers, ContextMenuEntry};
use crate::debloat::executor::{apply_debloat_preset, apply_debloat_rule};
use crate::debloat::rules::DebloatRule;
use crate::debloat::scanner::{scan_debloat_items, ScannedDebloatItem};
use crate::memory::optimizer::{optimize_memory, OptimizationReport};
use crate::monitoring::process::{list_running_processes, ProcessItem};
use crate::privacy::scanner::{scan_privacy_items, ScannedPrivacyItem};
use crate::restore::snapshots::{list_snapshots, Snapshot};
use crate::services::scanner::{scan_services, ServiceItem};
use crate::startup::scanner::{scan_startup_items, StartupItem};
use crate::tasks::scanner::{scan_scheduled_tasks, ScheduledTaskItem};
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
        }
    }
}

/// A one-shot mutating action executed off-thread.
#[derive(Debug, Clone)]
pub enum Action {
    ApplyDebloatPreset(String),
    ApplyDebloatRule(DebloatRule),
    OptimizeMemory(bool),
    CleanupFiles(Vec<ScannedCleanItem>),
}

#[derive(Debug, Clone)]
pub enum Job {
    Scan(ScanKind),
    Run(Action),
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
    MemoryReport(OptimizationReport),
    ActionDone { message: String, success: bool, follow_up: Option<ScanKind> },
    Event { category: String, message: String, is_success: bool },
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
        },
        Job::Run(action) => match action {
            Action::ApplyDebloatPreset(preset) => {
                let results = apply_debloat_preset(preset, false);
                JobResult::ActionDone {
                    message: format!("Applied '{}' preset ({} optimizations executed).", preset, results.len()),
                    success: true,
                    follow_up: Some(ScanKind::Debloat),
                }
            }
            Action::ApplyDebloatRule(rule) => {
                let results = apply_debloat_rule(rule, false);
                JobResult::ActionDone {
                    message: format!("Applied rule '{}' ({} steps executed).", rule.name, results.len()),
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
        },
    }
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

    EngineHandles { job_tx, job_rx: job_result_rx, auto_trim }
}
