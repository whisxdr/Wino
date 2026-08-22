use crate::app::navigation::NavTab;
use crate::app::worker::{Action, EngineHandles, Job, JobResult, ScanKind};
use crate::cleaner::scanner::ScannedCleanItem;
use crate::context_menu::scanner::ContextMenuEntry;
use crate::core::config::AppConfig;
use crate::core::i18n::{Lang};
use crate::core::system::SystemInfo;
use crate::debloat::scanner::ScannedDebloatItem;
use crate::health::diagnostics::{evaluate_system_health, SystemHealthReport};
use crate::memory::auto_trim::AutoTrimHandle;
use crate::memory::monitor::{capture_memory_snapshot, DetailedMemorySnapshot};
use crate::monitoring::cpu::get_cpu_usage;
use crate::monitoring::disk::get_primary_disk_stats;
use crate::monitoring::gpu::get_gpu_stats;
use crate::monitoring::network::get_network_stats;
use crate::monitoring::process::ProcessItem;
use crate::monitoring::ram::get_ram_stats;
use crate::monitoring::SystemMetricsSnapshot;
use crate::privacy::scanner::ScannedPrivacyItem;
use crate::restore::snapshots::Snapshot;
use crate::services::scanner::ServiceItem;
use crate::startup::scanner::StartupItem;
use crate::tasks::scanner::ScheduledTaskItem;
use chrono::Local;
use eframe::egui;
use std::collections::{HashSet, VecDeque};
use std::sync::mpsc::{Receiver, Sender};
use std::time::Instant;

/// Maximum number of samples retained in the sliding telemetry history.
pub const HISTORY_CAPACITY: usize = 24;

/// O(1) sliding-window insertion used by the live telemetry charts.
pub fn push_history_sample(buffer: &mut VecDeque<f32>, value: f32) {
    buffer.push_back(value);
    if buffer.len() > HISTORY_CAPACITY {
        buffer.pop_front();
    }
}

#[derive(Debug, Clone)]
pub struct RecentEvent {
    pub timestamp: String,
    pub category: String,
    pub message: String,
    pub is_success: bool,
    pub created_at: Instant,
}

pub struct AppState {
    pub config: AppConfig,
    pub sys_info: SystemInfo,
    pub current_tab: NavTab,
    pub lang: Lang,
    pub metrics: SystemMetricsSnapshot,
    pub memory_details: DetailedMemorySnapshot,
    pub health_report: SystemHealthReport,
    pub processes: Vec<ProcessItem>,
    pub debloat_items: Vec<ScannedDebloatItem>,
    pub startup_items: Vec<StartupItem>,
    pub services: Vec<ServiceItem>,
    pub privacy_items: Vec<ScannedPrivacyItem>,
    pub cleaner_items: Vec<ScannedCleanItem>,
    pub snapshots: Vec<Snapshot>,
    pub context_menu_items: Vec<ContextMenuEntry>,
    pub task_items: Vec<ScheduledTaskItem>,
    /// Selected DNS preset id for the Network view ("auto"/"cloudflare"/"google"/"quad9").
    pub dns_preset_id: String,
    pub search_query: String,
    pub debloat_preset: String,
    pub debloat_show_confirm: bool,
    pub debloat_confirm_preset: String,
    pub debloat_filter_preset: String,
    pub toast_message: Option<(String, Instant)>,
    pub last_event: Option<RecentEvent>,
    pub cpu_history: VecDeque<f32>,
    pub ram_history: VecDeque<f32>,
    pub last_fast_poll: Instant,
    pub last_slow_poll: Instant,

    // ---- Background engine ----
    job_tx: Sender<Job>,
    job_rx: Receiver<JobResult>,
    pub pending_scans: HashSet<ScanKind>,
    pub action_busy: bool,
    pub auto_trim: Option<AutoTrimHandle>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        let config = AppConfig::load();
        let sys_info = SystemInfo::detect();
        let memory_details = capture_memory_snapshot();
        let health_report = evaluate_system_health();
        let lang = Lang::from_code(&config.general.language);

        let initial_event = RecentEvent {
            timestamp: Local::now().format("%H:%M:%S").to_string(),
            category: "System".to_string(),
            message: "Wino Pro Engine initialized and ready".to_string(),
            is_success: true,
            created_at: Instant::now(),
        };

        let EngineHandles { job_tx, job_rx, auto_trim } =
            crate::app::worker::start_engine(config.memory.auto_trim_enabled);

        if let Some(handle) = &auto_trim {
            handle.threshold_pct.store(config.memory.auto_trim_threshold_pct.round() as u32, std::sync::atomic::Ordering::Release);
        }

        // Initialize with default history samples
        let initial_cpu: VecDeque<f32> = vec![10.0, 15.0, 12.0, 25.0, 40.0, 35.0, 50.0, 20.0, 30.0, 15.0, 10.0, 12.0, 18.0, 25.0, 20.0, 14.0].into();
        let initial_ram: VecDeque<f32> = vec![45.0; HISTORY_CAPACITY].into();

        Self {
            config,
            sys_info,
            current_tab: NavTab::Dashboard,
            lang,
            metrics: SystemMetricsSnapshot::default(),
            memory_details,
            health_report,
            processes: Vec::new(),
            debloat_items: Vec::new(),
            startup_items: Vec::new(),
            services: Vec::new(),
            privacy_items: Vec::new(),
            cleaner_items: Vec::new(),
            snapshots: Vec::new(),
            context_menu_items: Vec::new(),
            task_items: Vec::new(),
            dns_preset_id: "cloudflare".to_string(),
            search_query: String::new(),
            debloat_preset: "Safe".to_string(),
            debloat_show_confirm: false,
            debloat_confirm_preset: String::new(),
            debloat_filter_preset: "All".to_string(),
            toast_message: None,
            last_event: Some(initial_event),
            cpu_history: initial_cpu,
            ram_history: initial_ram,
            last_fast_poll: Instant::now(),
            last_slow_poll: Instant::now(),

            job_tx,
            job_rx,
            pending_scans: HashSet::new(),
            action_busy: false,
            auto_trim,
        }
    }

    // ================= Background engine plumbing =================

    /// Queue a scan on a worker thread (no-op while that scan is in flight).
    pub fn request_scan(&mut self, kind: ScanKind) {
        if self.pending_scans.contains(&kind) {
            return;
        }
        self.pending_scans.insert(kind);
        let _ = self.job_tx.send(Job::Scan(kind));
        self.record_event("Scan", &format!("{} {}...", crate::core::i18n::tr(self.lang, "common.scanning"), kind.label()), true);
    }

    /// Queue a mutating action (one at a time).
    pub fn request_action(&mut self, action: Action) {
        if self.action_busy {
            self.set_toast("Another operation is still running — please wait.");
            return;
        }
        self.action_busy = true;
        let _ = self.job_tx.send(Job::Run(action));
    }

    /// Drain finished results from the worker pool (called every frame).
    fn poll_results(&mut self) {
        while let Ok(result) = self.job_rx.try_recv() {
            match result {
                JobResult::ScannedProcesses(items) => {
                    self.processes = items.clone();
                    self.finish_scan(ScanKind::Processes, format!("Refreshed {} active processes", items.len()));
                }
                JobResult::ScannedDebloat(items) => {
                    self.debloat_items = items.clone();
                    self.finish_scan(ScanKind::Debloat, format!("Scanned {} debloat targets", items.len()));
                }
                JobResult::ScannedStartup(items) => {
                    self.startup_items = items.clone();
                    self.finish_scan(ScanKind::Startup, format!("Scanned {} startup entries", items.len()));
                }
                JobResult::ScannedServices(items) => {
                    self.services = items.clone();
                    self.finish_scan(ScanKind::Services, format!("Loaded {} service configurations", items.len()));
                }
                JobResult::ScannedPrivacy(items) => {
                    self.privacy_items = items.clone();
                    self.finish_scan(ScanKind::Privacy, format!("Scanned {} privacy telemetry rules", items.len()));
                }
                JobResult::ScannedCleaner(items) => {
                    let total_mb: u64 = items.iter().map(|i| i.total_bytes).sum::<u64>() / (1024 * 1024);
                    self.cleaner_items = items.clone();
                    self.finish_scan(ScanKind::Cleaner, format!("Scanned temporary storage (~{} MB recoverable)", total_mb));
                }
                JobResult::ScannedSnapshots(items) => {
                    self.snapshots = items.clone();
                    self.finish_scan(ScanKind::Snapshots, format!("Loaded {} system snapshots", items.len()));
                }
                JobResult::ScannedContextMenu(items) => {
                    self.context_menu_items = items.clone();
                    self.finish_scan(ScanKind::ContextMenu, format!("Found {} shell context-menu handlers", items.len()));
                }
                JobResult::ScannedTasks(items) => {
                    self.task_items = items.clone();
                    self.finish_scan(ScanKind::Tasks, format!("Found {} telemetry/updater scheduled tasks", items.len()));
                }
                JobResult::MemoryReport(report) => {
                    self.action_busy = false;
                    self.set_toast(&report.message);
                }
                JobResult::ActionDone { message, success, follow_up } => {
                    self.action_busy = false;
                    if success {
                        self.set_toast(&message);
                    } else {
                        self.record_event("Action", &message, false);
                        self.set_toast(&message);
                    }
                    if let Some(kind) = follow_up {
                        self.request_scan(kind);
                    }
                }
                JobResult::Event { category, message, is_success } => {
                    self.record_event(&category, &message, is_success);
                }
            }
        }
    }

    fn finish_scan(&mut self, kind: ScanKind, message: String) {
        self.pending_scans.remove(&kind);
        self.record_event(kind.label(), &message, true);
    }

    // ================= Legacy public API (now async-backed) =================

    pub fn set_toast(&mut self, msg: &str) {
        self.toast_message = Some((msg.to_string(), Instant::now()));
        self.record_event("Action", msg, true);
    }

    pub fn record_event(&mut self, category: &str, msg: &str, is_success: bool) {
        let event = RecentEvent {
            timestamp: Local::now().format("%H:%M:%S").to_string(),
            category: category.to_string(),
            message: msg.to_string(),
            is_success,
            created_at: Instant::now(),
        };
        self.last_event = Some(event);
    }

    pub fn update_tick(&mut self, ctx: &egui::Context) {
        let now = Instant::now();

        // Drain completed background jobs first so fresh data paints this frame.
        self.poll_results();

        // 1. Fast polling for CPU & RAM (500ms when focused)
        if now.duration_since(self.last_fast_poll).as_millis() >= 500 {
            self.last_fast_poll = now;
            let cpu = get_cpu_usage();
            let ram = get_ram_stats();
            let net = get_network_stats();
            let disk = get_primary_disk_stats();
            let gpu = get_gpu_stats();

            self.metrics = SystemMetricsSnapshot {
                cpu_usage_pct: cpu,
                ram_total_bytes: ram.total_bytes,
                ram_used_bytes: ram.used_bytes,
                ram_available_bytes: ram.available_bytes,
                ram_cached_bytes: ram.cached_bytes,
                ram_usage_pct: ram.usage_pct,
                commit_used_bytes: ram.commit_used_bytes,
                commit_limit_bytes: ram.commit_limit_bytes,
                disk_total_bytes: disk.total_bytes,
                disk_free_bytes: disk.free_bytes,
                disk_usage_pct: disk.usage_pct,
                net_recv_bytes_per_sec: net.bytes_recv_per_sec,
                net_send_bytes_per_sec: net.bytes_send_per_sec,
                gpu_name: gpu.name,
                gpu_vram_total_bytes: gpu.dedicated_vram_bytes,
                gpu_vram_used_bytes: gpu.shared_system_memory_bytes,
                process_count: ram.process_count,
            };

            // O(1) sliding history buffers — no heap-shifting Vec::remove(0).
            push_history_sample(&mut self.cpu_history, cpu);
            push_history_sample(&mut self.ram_history, ram.usage_pct);

            self.memory_details = capture_memory_snapshot();
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }

        // 2. Slow polling / on-demand refresh
        if now.duration_since(self.last_slow_poll).as_secs() >= 5 {
            self.last_slow_poll = now;
            self.health_report = evaluate_system_health();
        }

        // Clear toast after 4 seconds
        if let Some((_, time)) = self.toast_message {
            if now.duration_since(time).as_secs() >= 4 {
                self.toast_message = None;
            }
        }

        // Keep repainting while background work is outstanding.
        if self.is_busy() {
            ctx.request_repaint_after(std::time::Duration::from_millis(120));
        }
    }

    pub fn is_busy(&self) -> bool {
        self.action_busy || !self.pending_scans.is_empty()
    }

    pub fn refresh_processes(&mut self) {
        self.request_scan(ScanKind::Processes);
    }

    pub fn refresh_debloat(&mut self) {
        self.request_scan(ScanKind::Debloat);
    }

    pub fn refresh_startup(&mut self) {
        self.request_scan(ScanKind::Startup);
    }

    pub fn refresh_services(&mut self) {
        self.request_scan(ScanKind::Services);
    }

    pub fn refresh_privacy(&mut self) {
        self.request_scan(ScanKind::Privacy);
    }

    pub fn refresh_cleaner(&mut self) {
        self.request_scan(ScanKind::Cleaner);
    }

    pub fn refresh_snapshots(&mut self) {
        self.request_scan(ScanKind::Snapshots);
    }

    pub fn refresh_context_menu(&mut self) {
        self.request_scan(ScanKind::ContextMenu);
    }

    pub fn refresh_tasks(&mut self) {
        self.request_scan(ScanKind::Tasks);
    }

    /// Persist language changes and notify the UI.
    pub fn set_language(&mut self, lang: Lang) {
        self.lang = lang;
        self.config.general.language = lang.code().to_string();
        let _ = self.config.save();
    }

    /// Sync auto-trimmer runtime knobs with the config panel.
    pub fn apply_auto_trim_settings(&mut self) {
        if let Some(handle) = &self.auto_trim {
            handle.set_enabled(self.config.memory.auto_trim_enabled);
            handle.threshold_pct.store(
                self.config.memory.auto_trim_threshold_pct.round() as u32,
                std::sync::atomic::Ordering::Release,
            );
        }
        let _ = self.config.save();
    }
}
