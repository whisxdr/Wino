use crate::app::navigation::NavTab;
use crate::app::worker::{Action, EngineHandles, Job, JobResult, ScanKind};
use crate::apps::models::{AppRecord, AppSort, AppSource, WingetUpdate};
use crate::benchmark::models::BenchmarkSample;
use crate::cleaner::scanner::ScannedCleanItem;
use crate::context_menu::scanner::ContextMenuEntry;
use crate::core::cancel::CancelToken;
use crate::core::config::AppConfig;
use crate::core::i18n::Lang;
use crate::core::system::SystemInfo;
use crate::debloat::scanner::ScannedDebloatItem;
use crate::health::center::HealthReport;
use crate::health::diagnostics::SystemHealthReport;
use crate::memory::auto_trim::AutoTrimHandle;
use crate::memory::monitor::{capture_memory_snapshot, DetailedMemorySnapshot};
use crate::monitoring::cpu::get_cpu_usage;
use crate::monitoring::disk::get_primary_disk_stats;
use crate::monitoring::gpu::get_gpu_stats;
use crate::monitoring::network::get_network_stats;
use crate::monitoring::process::ProcessItem;
use crate::monitoring::ram::get_ram_stats;
use crate::monitoring::SystemMetricsSnapshot;
use crate::network::diagnostics::NetworkReport;
use crate::power::manager::PowerPlan;
use crate::power::settings::PowerSetting;
use crate::privacy::scanner::ScannedPrivacyItem;
use crate::profiles::models::Profile;
use crate::recommendations::models::Recommendation;
use crate::restore::snapshots::Snapshot;
use crate::security::center::SecurityReport;
use crate::services::scanner::ServiceItem;
use crate::startup::scanner::StartupItem;
use crate::storage::models::StorageTree;
use crate::tasks::scanner::ScheduledTaskItem;
use crate::windows_features::models::WindowsFeature;
use chrono::Local;
use eframe::egui;
use std::collections::{HashMap, HashSet, VecDeque};
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

/// A destructive action the user has confirmed but which is waiting for the
/// shared confirmation dialog to be accepted.
///
/// Every mutating action funnels through here so no view can apply a change
/// without the disclosure step.
#[derive(Debug, Clone)]
pub enum PendingConfirm {
    UninstallApp(Box<AppRecord>),
    ApplyProfile(Box<Profile>),
    SetPowerPlan {
        plan_id: String,
        label: String,
    },
    SetPowerSetting {
        setting: Box<PowerSetting>,
        ac: u32,
        dc: u32,
    },
    ToggleFeature {
        name: String,
        enable: bool,
        label: String,
    },
    ToggleStartup {
        item: Box<StartupItem>,
        enable: bool,
    },
    RollbackSnapshot {
        id: String,
        label: String,
    },
    DeleteSnapshot {
        id: String,
        label: String,
    },
    UpdateWinget {
        package_ids: Vec<String>,
        label: String,
    },
}

impl PendingConfirm {
    /// Dialog title key suffix shared by the confirmation modal.
    pub fn title(&self, lang: Lang) -> String {
        use crate::core::i18n::tr;
        match self {
            PendingConfirm::UninstallApp(rec) => {
                format!(
                    "{}: {}",
                    tr(lang, "apps.uninstall_confirm_title"),
                    rec.display_name
                )
            }
            PendingConfirm::ApplyProfile(p) => {
                format!("{}: {}", tr(lang, "profiles.confirm_apply_title"), p.name)
            }
            PendingConfirm::SetPowerPlan { label, .. } => {
                format!("{}: {}", tr(lang, "power.apply_plan"), label)
            }
            PendingConfirm::SetPowerSetting { setting, .. } => {
                format!("{}: {}", tr(lang, "power.settings_title"), setting.name)
            }
            PendingConfirm::ToggleFeature { label, enable, .. } => {
                let verb = if *enable {
                    tr(lang, "features.confirm_enable")
                } else {
                    tr(lang, "features.confirm_disable")
                };
                format!("{}: {}", verb, label)
            }
            PendingConfirm::ToggleStartup { item, enable } => {
                let verb = if *enable {
                    tr(lang, "common.enable")
                } else {
                    tr(lang, "common.disable")
                };
                format!("{}: {}", verb, item.name)
            }
            PendingConfirm::RollbackSnapshot { label, .. } => {
                format!("{}: {}", tr(lang, "restore.restore_confirm_title"), label)
            }
            PendingConfirm::DeleteSnapshot { label, .. } => {
                format!("{}: {}", tr(lang, "restore.delete_snapshot"), label)
            }
            PendingConfirm::UpdateWinget { label, .. } => {
                format!("{}: {}", tr(lang, "apps.update_selected"), label)
            }
        }
    }

    /// Body text explaining what the confirmed action does.
    pub fn body(&self, lang: Lang) -> String {
        use crate::core::i18n::tr;
        match self {
            PendingConfirm::UninstallApp(_) => tr(lang, "apps.uninstall_confirm_body").to_string(),
            PendingConfirm::ApplyProfile(_) => tr(lang, "profiles.confirm_apply_body").to_string(),
            PendingConfirm::SetPowerPlan { .. } => tr(lang, "power.plan_applied").to_string(),
            PendingConfirm::SetPowerSetting { .. } => {
                tr(lang, "power.settings_subtitle").to_string()
            }
            PendingConfirm::ToggleFeature { .. } => tr(lang, "features.warning_body").to_string(),
            PendingConfirm::ToggleStartup { .. } => tr(lang, "su.approved_note").to_string(),
            PendingConfirm::RollbackSnapshot { .. } => {
                tr(lang, "restore.restore_confirm_body").to_string()
            }
            PendingConfirm::DeleteSnapshot { .. } => tr(lang, "restore.delete_confirm").to_string(),
            PendingConfirm::UpdateWinget { .. } => tr(lang, "apps.updates_subtitle").to_string(),
        }
    }

    /// Reversibility note shown in the dialog.
    pub fn reversible(&self) -> bool {
        !matches!(
            self,
            PendingConfirm::UninstallApp(_) | PendingConfirm::DeleteSnapshot { .. }
        )
    }

    /// Whether the operation needs Administrator privileges.
    pub fn requires_admin(&self) -> bool {
        matches!(
            self,
            PendingConfirm::ToggleFeature { .. }
                | PendingConfirm::SetPowerPlan { .. }
                | PendingConfirm::SetPowerSetting { .. }
        )
    }

    /// Convert the confirmed choice into the worker action to run.
    pub fn into_action(self) -> Action {
        match self {
            PendingConfirm::UninstallApp(record) => Action::UninstallApp {
                record,
                dry_run: false,
            },
            PendingConfirm::ApplyProfile(profile) => Action::ApplyProfile {
                profile,
                dry_run: false,
            },
            PendingConfirm::SetPowerPlan { plan_id, .. } => Action::SetPowerPlan {
                plan_id,
                dry_run: false,
            },
            PendingConfirm::SetPowerSetting { setting, ac, dc } => Action::SetPowerSetting {
                setting,
                ac_value: ac,
                dc_value: dc,
                dry_run: false,
            },
            PendingConfirm::ToggleFeature { name, enable, .. } => Action::ToggleWindowsFeature {
                feature_name: name,
                enable,
                dry_run: false,
            },
            PendingConfirm::ToggleStartup { item, enable } => Action::ToggleStartupEntry {
                item,
                enable,
                dry_run: false,
            },
            PendingConfirm::RollbackSnapshot { id, .. } => {
                Action::RollbackSnapshot { snapshot_id: id }
            }
            PendingConfirm::DeleteSnapshot { id, .. } => Action::DeleteSnapshot { snapshot_id: id },
            PendingConfirm::UpdateWinget { package_ids, .. } => {
                if package_ids.len() == 1 {
                    Action::UpdateWingetPackage {
                        package_id: package_ids[0].clone(),
                        dry_run: false,
                    }
                } else {
                    Action::UpdateAllWinget { dry_run: false }
                }
            }
        }
    }
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
    /// Selected DNS preset id for the Network view ("auto"/"cloudflare"/"google"/"quad9"/"adguard").
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

    // ---- v2.6: Applications ----
    pub apps: Vec<AppRecord>,
    pub app_updates: Vec<WingetUpdate>,
    pub apps_source_filter: Option<AppSource>,
    pub apps_sort: AppSort,
    pub apps_search: String,
    pub apps_selected: HashSet<String>,
    pub winget_status: crate::apps::models::WingetStatus,

    // ---- v2.6: Profiles ----
    pub profiles: Vec<Profile>,
    pub profiles_selected: Option<String>,
    /// Set while a profile is being renamed; holds the working copy.
    pub profile_editor: Option<Profile>,
    pub profile_rename_buffer: String,
    /// Path typed into the profile import field.
    pub profile_import_buffer: String,
    /// Profile id whose Safety Engine preview is expanded.
    pub profile_preview_id: Option<String>,

    // ---- v2.6: Power ----
    pub power_plans: Vec<PowerPlan>,
    pub power_settings: Vec<PowerSetting>,
    pub power_active_guid: String,
    pub power_edit_ac: HashMap<String, u32>,
    pub power_edit_dc: HashMap<String, u32>,

    // ---- v2.6: Windows features ----
    pub features: Vec<WindowsFeature>,
    pub features_search: String,

    // ---- v2.6: Network ----
    pub network_report: NetworkReport,
    pub network_host_input: String,
    pub network_tool_result: Option<(String, bool)>,

    // ---- v2.6: Health center ----
    pub health_center: HealthReport,
    pub health_tool_output: Option<(String, String, bool)>,

    // ---- v2.6: Security ----
    pub security_report: SecurityReport,

    // ---- v2.6: Storage ----
    pub storage_tree: Option<StorageTree>,
    pub storage_scan_cancel: CancelToken,
    pub storage_current_path: Option<String>,

    // ---- v2.6: Recommendations ----
    pub recommendations: Vec<Recommendation>,

    // ---- v2.6: Benchmark ----
    pub benchmark_samples: Vec<BenchmarkSample>,
    pub benchmark_selected: Vec<String>,

    // ---- v2.6: Confirmations ----
    pub pending_confirm: Option<PendingConfirm>,

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
        let health_report = evaluate_system_health_shim();
        let lang = Lang::from_code(&config.general.language);

        let initial_event = RecentEvent {
            timestamp: Local::now().format("%H:%M:%S").to_string(),
            category: "System".to_string(),
            message: "Wino 2.6 engine initialized and ready".to_string(),
            is_success: true,
            created_at: Instant::now(),
        };

        let EngineHandles {
            job_tx,
            job_rx,
            auto_trim,
        } = crate::app::worker::start_engine(config.memory.auto_trim_enabled);

        if let Some(handle) = &auto_trim {
            handle.threshold_pct.store(
                config.memory.auto_trim_threshold_pct.round() as u32,
                std::sync::atomic::Ordering::Release,
            );
        }

        let initial_cpu: VecDeque<f32> = vec![
            10.0, 15.0, 12.0, 25.0, 40.0, 35.0, 50.0, 20.0, 30.0, 15.0, 10.0, 12.0, 18.0, 25.0,
            20.0, 14.0,
        ]
        .into();
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

            apps: Vec::new(),
            app_updates: Vec::new(),
            apps_source_filter: None,
            apps_sort: AppSort::Name,
            apps_search: String::new(),
            apps_selected: HashSet::new(),
            winget_status: crate::apps::models::WingetStatus::missing("Not checked yet."),

            profiles: Vec::new(),
            profiles_selected: None,
            profile_editor: None,
            profile_rename_buffer: String::new(),
            profile_import_buffer: String::new(),
            profile_preview_id: None,

            power_plans: Vec::new(),
            power_settings: Vec::new(),
            power_active_guid: String::new(),
            power_edit_ac: HashMap::new(),
            power_edit_dc: HashMap::new(),

            features: Vec::new(),
            features_search: String::new(),

            network_report: NetworkReport::default(),
            network_host_input: String::new(),
            network_tool_result: None,

            health_center: HealthReport::default(),
            health_tool_output: None,

            security_report: SecurityReport::default(),

            storage_tree: None,
            storage_scan_cancel: CancelToken::new(),
            storage_current_path: None,

            recommendations: Vec::new(),

            benchmark_samples: Vec::new(),
            benchmark_selected: Vec::new(),

            pending_confirm: None,

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
        self.record_event(
            "Scan",
            &format!(
                "{} {}...",
                crate::core::i18n::tr(self.lang, "common.scanning"),
                kind.label()
            ),
            true,
        );
    }

    /// Queue a mutating action (one at a time).
    pub fn request_action(&mut self, action: Action) {
        if self.action_busy {
            self.set_toast("Another operation is still running. Wait for it to finish.");
            return;
        }
        self.action_busy = true;
        let _ = self.job_tx.send(Job::Run(Box::new(action)));
    }

    /// Stage a destructive action for confirmation instead of running it.
    pub fn request_confirm(&mut self, confirm: PendingConfirm) {
        if self.action_busy {
            self.set_toast("Another operation is still running. Wait for it to finish.");
            return;
        }
        self.pending_confirm = Some(confirm);
    }

    /// Accept the staged confirmation and dispatch its action.
    pub fn confirm_pending(&mut self) {
        if let Some(pending) = self.pending_confirm.take() {
            self.request_action(pending.into_action());
        }
    }

    /// Discard the staged confirmation.
    pub fn cancel_pending(&mut self) {
        self.pending_confirm = None;
    }

    /// Drain finished results from the worker pool (called every frame).
    fn poll_results(&mut self) {
        while let Ok(result) = self.job_rx.try_recv() {
            match result {
                JobResult::ScannedProcesses(items) => {
                    self.processes = items.clone();
                    self.finish_scan(
                        ScanKind::Processes,
                        format!("Refreshed {} active processes", items.len()),
                    );
                }
                JobResult::ScannedDebloat(items) => {
                    self.debloat_items = items.clone();
                    self.finish_scan(
                        ScanKind::Debloat,
                        format!("Scanned {} debloat targets", items.len()),
                    );
                }
                JobResult::ScannedStartup(items) => {
                    self.startup_items = items.clone();
                    self.finish_scan(
                        ScanKind::Startup,
                        format!("Scanned {} startup entries", items.len()),
                    );
                }
                JobResult::ScannedServices(items) => {
                    self.services = items.clone();
                    self.finish_scan(
                        ScanKind::Services,
                        format!("Loaded {} service configurations", items.len()),
                    );
                }
                JobResult::ScannedPrivacy(items) => {
                    self.privacy_items = items.clone();
                    self.finish_scan(
                        ScanKind::Privacy,
                        format!("Scanned {} privacy telemetry rules", items.len()),
                    );
                }
                JobResult::ScannedCleaner(items) => {
                    let total_mb: u64 =
                        items.iter().map(|i| i.total_bytes).sum::<u64>() / (1024 * 1024);
                    self.cleaner_items = items.clone();
                    self.finish_scan(
                        ScanKind::Cleaner,
                        format!("Scanned temporary storage (~{} MB recoverable)", total_mb),
                    );
                }
                JobResult::ScannedSnapshots(items) => {
                    self.snapshots = items.clone();
                    self.finish_scan(
                        ScanKind::Snapshots,
                        format!("Loaded {} system snapshots", items.len()),
                    );
                }
                JobResult::ScannedContextMenu(items) => {
                    self.context_menu_items = items.clone();
                    self.finish_scan(
                        ScanKind::ContextMenu,
                        format!("Found {} shell context-menu handlers", items.len()),
                    );
                }
                JobResult::ScannedTasks(items) => {
                    self.task_items = items.clone();
                    self.finish_scan(
                        ScanKind::Tasks,
                        format!("Found {} telemetry/updater scheduled tasks", items.len()),
                    );
                }
                JobResult::ScannedApps(items) => {
                    self.apps = items.clone();
                    self.apps_selected
                        .retain(|name| items.iter().any(|a| &a.name == name));
                    self.finish_scan(
                        ScanKind::Apps,
                        format!("Discovered {} installed applications", items.len()),
                    );
                }
                JobResult::ScannedAppUpdates(items) => {
                    self.app_updates = items.clone();
                    self.finish_scan(
                        ScanKind::AppUpdates,
                        format!("{} package update(s) reported by Winget", items.len()),
                    );
                }
                JobResult::ScannedProfiles(items) => {
                    self.profiles = items.clone();
                    if self.profiles_selected.is_none() {
                        self.profiles_selected = items.first().map(|p| p.id.clone());
                    }
                    self.finish_scan(
                        ScanKind::Profiles,
                        format!("Loaded {} profiles", items.len()),
                    );
                }
                JobResult::ScannedPower { plans, settings } => {
                    self.power_active_guid = plans
                        .iter()
                        .find(|p| p.is_active)
                        .map(|p| p.guid.clone())
                        .unwrap_or_default();
                    self.power_edit_ac.clear();
                    self.power_edit_dc.clear();
                    for s in &settings {
                        if let Some(ac) = s.ac_value {
                            self.power_edit_ac.insert(s.id.clone(), ac);
                        }
                        if let Some(dc) = s.dc_value {
                            self.power_edit_dc.insert(s.id.clone(), dc);
                        }
                    }
                    self.power_plans = plans;
                    self.power_settings = settings;
                    self.finish_scan(
                        ScanKind::Power,
                        "Loaded power plans and advanced settings".to_string(),
                    );
                }
                JobResult::ScannedFeatures(items) => {
                    self.features = items.clone();
                    let enabled = items
                        .iter()
                        .filter(|f| {
                            f.state == crate::windows_features::models::FeatureState::Enabled
                        })
                        .count();
                    self.finish_scan(
                        ScanKind::Features,
                        format!(
                            "{} optional features reported ({} enabled)",
                            items.len(),
                            enabled
                        ),
                    );
                }
                JobResult::ScannedNetwork(report) => {
                    let label = if report.active_adapter.is_empty() {
                        "No active adapter".to_string()
                    } else {
                        report.active_adapter.clone()
                    };
                    self.network_report = *report;
                    self.finish_scan(
                        ScanKind::Network,
                        format!("Network state refreshed ({})", label),
                    );
                }
                JobResult::ScannedHealth(report) => {
                    self.health_center = *report;
                    self.finish_scan(
                        ScanKind::Health,
                        format!(
                            "Health scan complete: {}",
                            self.health_center.overall.as_str()
                        ),
                    );
                }
                JobResult::ScannedSecurity(report) => {
                    self.security_report = *report;
                    self.finish_scan(
                        ScanKind::Security,
                        "Security center state refreshed".to_string(),
                    );
                }
                JobResult::ScannedStorage(tree) => {
                    let files = tree.files_scanned;
                    let partial = tree.limit_reached;
                    self.storage_tree = Some(*tree);
                    self.finish_scan(
                        ScanKind::Storage,
                        if partial {
                            format!(
                                "Storage scan stopped at the scan limit after {} files",
                                files
                            )
                        } else {
                            format!("Storage scan complete ({} files)", files)
                        },
                    );
                }
                JobResult::ScannedRecommendations(items) => {
                    self.recommendations = items.clone();
                    self.finish_scan(
                        ScanKind::Recommendations,
                        format!("{} recommendation(s) from measured state", items.len()),
                    );
                }
                JobResult::Benchmarked(sample) => {
                    self.benchmark_samples.push(sample.clone());
                    self.action_busy = false;
                    self.set_toast(&format!("Captured benchmark '{}'.", sample.label));
                }
                JobResult::NetworkToolDone {
                    title,
                    message,
                    success,
                } => {
                    self.action_busy = false;
                    self.network_tool_result = Some((format!("{} — {}", title, message), success));
                    if !success {
                        self.record_event("Network", &format!("{} failed", title), false);
                    }
                }
                JobResult::ProfileApplied(report) => {
                    self.action_busy = false;
                    if report.is_partial() {
                        self.record_event("Profiles", &report.message, false);
                    }
                    self.set_toast(&report.message);
                    self.request_scan(ScanKind::Profiles);
                }
                JobResult::MemoryReport(report) => {
                    self.action_busy = false;
                    self.set_toast(&report.message);
                }
                JobResult::ToolOutput {
                    title,
                    detail,
                    success,
                    partial,
                } => {
                    self.action_busy = false;
                    self.health_tool_output = Some((title.clone(), detail.clone(), success));
                    let summary = if partial {
                        format!("{} completed with partial failures.", title)
                    } else if success {
                        format!("{} finished.", title)
                    } else {
                        format!("{} failed.", title)
                    };
                    if success {
                        self.set_toast(&summary);
                    } else {
                        self.record_event("Tool", &summary, false);
                        self.set_toast(&summary);
                    }
                }
                JobResult::ActionDone {
                    message,
                    success,
                    follow_up,
                } => {
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
                JobResult::Event {
                    category,
                    message,
                    is_success,
                } => {
                    self.record_event(&category, &message, is_success);
                }
            }
        }
    }

    fn finish_scan(&mut self, kind: ScanKind, message: String) {
        self.pending_scans.remove(&kind);
        self.record_event(kind.label(), &message, true);
    }

    // ================= Public helpers used by views =================

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

            push_history_sample(&mut self.cpu_history, cpu);
            push_history_sample(&mut self.ram_history, ram.usage_pct);

            self.memory_details = capture_memory_snapshot();
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }

        // 2. Slow polling / on-demand refresh
        if now.duration_since(self.last_slow_poll).as_secs() >= 5 {
            self.last_slow_poll = now;
            self.health_report = evaluate_system_health_shim();
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

    pub fn is_scanning(&self, kind: ScanKind) -> bool {
        self.pending_scans.contains(&kind)
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

    pub fn refresh_apps(&mut self) {
        self.request_scan(ScanKind::Apps);
    }

    pub fn refresh_app_updates(&mut self) {
        self.request_scan(ScanKind::AppUpdates);
    }

    pub fn refresh_profiles(&mut self) {
        self.request_scan(ScanKind::Profiles);
    }

    pub fn refresh_power(&mut self) {
        self.request_scan(ScanKind::Power);
    }

    pub fn refresh_features(&mut self) {
        self.request_scan(ScanKind::Features);
    }

    pub fn refresh_network(&mut self) {
        self.request_scan(ScanKind::Network);
    }

    pub fn refresh_health_center(&mut self) {
        self.request_scan(ScanKind::Health);
    }

    pub fn refresh_security(&mut self) {
        self.request_scan(ScanKind::Security);
    }

    pub fn refresh_storage(&mut self) {
        self.request_scan(ScanKind::Storage);
    }

    pub fn refresh_recommendations(&mut self) {
        self.request_scan(ScanKind::Recommendations);
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

    /// Save the whole config and report success to the user.
    pub fn save_config(&mut self) {
        match self.config.save() {
            Ok(()) => self.set_toast(crate::core::i18n::tr(self.lang, "set.saved")),
            Err(e) => self.record_event(
                "Config",
                &format!("Failed to save configuration: {}", e),
                false,
            ),
        }
    }
}

/// The health center supersedes the v2.5 dashboard health summary, but the
/// dashboard still shows the compact rating. Keeping the call behind one
/// helper avoids two divergent definitions of "healthy".
fn evaluate_system_health_shim() -> SystemHealthReport {
    crate::health::diagnostics::evaluate_system_health()
}
