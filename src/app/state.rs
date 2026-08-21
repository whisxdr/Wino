use crate::app::navigation::NavTab;
use crate::cleaner::scanner::{scan_cleaner_targets, ScannedCleanItem};
use crate::core::config::AppConfig;
use crate::core::system::SystemInfo;
use crate::debloat::scanner::{scan_debloat_items, ScannedDebloatItem};
use crate::health::diagnostics::{evaluate_system_health, SystemHealthReport};
use crate::memory::monitor::{capture_memory_snapshot, DetailedMemorySnapshot};
use crate::monitoring::cpu::get_cpu_usage;
use crate::monitoring::disk::get_primary_disk_stats;
use crate::monitoring::gpu::get_gpu_stats;
use crate::monitoring::network::get_network_stats;
use crate::monitoring::process::{list_running_processes, ProcessItem};
use crate::monitoring::ram::get_ram_stats;
use crate::monitoring::SystemMetricsSnapshot;
use crate::privacy::scanner::{scan_privacy_items, ScannedPrivacyItem};
use crate::restore::snapshots::{list_snapshots, Snapshot};
use crate::services::scanner::{scan_services, ServiceItem};
use crate::startup::scanner::{scan_startup_items, StartupItem};
use chrono::Local;
use eframe::egui;
use std::time::Instant;

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
    pub search_query: String,
    pub debloat_preset: String,
    pub debloat_show_confirm: bool,
    pub debloat_confirm_preset: String,
    pub debloat_filter_preset: String,
    pub toast_message: Option<(String, Instant)>,
    pub last_event: Option<RecentEvent>,
    pub cpu_history: Vec<f32>,
    pub ram_history: Vec<f32>,
    pub last_fast_poll: Instant,
    pub last_slow_poll: Instant,
}

impl AppState {
    pub fn new() -> Self {
        let config = AppConfig::load();
        let sys_info = SystemInfo::detect();
        let memory_details = capture_memory_snapshot();
        let health_report = evaluate_system_health();

        let initial_event = RecentEvent {
            timestamp: Local::now().format("%H:%M:%S").to_string(),
            category: "System".to_string(),
            message: "Wino Pro Engine initialized and ready".to_string(),
            is_success: true,
            created_at: Instant::now(),
        };

        // Initialize with default history samples
        let initial_cpu = vec![10.0, 15.0, 12.0, 25.0, 40.0, 35.0, 50.0, 20.0, 30.0, 15.0, 10.0, 12.0, 18.0, 25.0, 20.0, 14.0];
        let initial_ram = vec![45.0; 16];

        Self {
            config,
            sys_info,
            current_tab: NavTab::Dashboard,
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
        }
    }

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

            // Update sliding history buffers (keep up to 24 samples)
            self.cpu_history.push(cpu);
            if self.cpu_history.len() > 24 {
                self.cpu_history.remove(0);
            }

            self.ram_history.push(ram.usage_pct);
            if self.ram_history.len() > 24 {
                self.ram_history.remove(0);
            }

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
    }

    pub fn refresh_processes(&mut self) {
        self.processes = list_running_processes();
        self.record_event("Processes", &format!("Refreshed {} active processes", self.processes.len()), true);
    }

    pub fn refresh_debloat(&mut self) {
        self.debloat_items = scan_debloat_items();
        self.record_event("Debloat", &format!("Scanned {} debloat targets", self.debloat_items.len()), true);
    }

    pub fn refresh_startup(&mut self) {
        self.startup_items = scan_startup_items();
        self.record_event("Startup", &format!("Scanned {} startup entries", self.startup_items.len()), true);
    }

    pub fn refresh_services(&mut self) {
        self.services = scan_services();
        self.record_event("Services", &format!("Loaded {} service configurations", self.services.len()), true);
    }

    pub fn refresh_privacy(&mut self) {
        self.privacy_items = scan_privacy_items();
        self.record_event("Privacy", &format!("Scanned {} privacy telemetry rules", self.privacy_items.len()), true);
    }

    pub fn refresh_cleaner(&mut self) {
        self.cleaner_items = scan_cleaner_targets();
        let total_mb: u64 = self.cleaner_items.iter().map(|i| i.total_bytes).sum::<u64>() / (1024 * 1024);
        self.record_event("Cleaner", &format!("Scanned temporary storage (~{} MB recoverable)", total_mb), true);
    }

    pub fn refresh_snapshots(&mut self) {
        self.snapshots = list_snapshots();
        self.record_event("Restore", &format!("Loaded {} system snapshots", self.snapshots.len()), true);
    }
}
