//! Dual-language support: English (EN) & Bahasa Indonesia (ID).
//!
//! UI chrome (labels, titles, buttons, dialogs, toasts) covers both languages.
//! System identifiers, service names (DiagTrack, SysMain), AppX package
//! names, registry paths intentionally remain in the original English data
//! layer for technical accuracy and reliable search filtering.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lang {
    En,
    Id,
}

impl Lang {
    pub fn from_code(code: &str) -> Self {
        match code {
            "id" => Lang::Id,
            _ => Lang::En,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Id => "id",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Id => "Bahasa Indonesia",
        }
    }

    pub fn toggle(&self) -> Self {
        match self {
            Lang::En => Lang::Id,
            Lang::Id => Lang::En,
        }
    }
}

/// Every translatable key. The parity unit test guarantees both dictionaries
/// stay complete.
pub const KEYS: &[&str] = &[
    // Navigation
    "nav.dashboard", "nav.memory", "nav.processes", "nav.debloat", "nav.startup",
    "nav.services", "nav.privacy", "nav.cleaner", "nav.gaming", "nav.health",
    "nav.restore", "nav.contextmenu", "nav.network", "nav.tasks", "nav.logs", "nav.settings",
    // Common
    "common.apply", "common.rescan", "common.scan_now", "common.cancel",
    "common.dry_run_test", "common.search", "common.filter_view", "common.enabled",
    "common.disabled", "common.loading", "common.benefit", "common.target", "common.all",
    "common.optimized", "common.admin_elevated", "common.standard_user",
    "common.scanning", "common.status_ready", "common.live_badge",
    // Dashboard
    "dash.title", "dash.subtitle", "dash.refresh_telemetry", "dash.cpu_telemetry",
    "dash.host_info", "dash.active_load", "dash.cores_threads", "dash.memory_engine",
    "dash.storage_status", "dash.quick_trim", "dash.inspect_engine", "dash.scan_junk",
    "dash.health_actions", "dash.protected_ok", "dash.attention_needed",
    "dash.av_active_title", "dash.av_active_desc", "dash.av_off_title", "dash.av_off_desc",
    "dash.open_health", "dash.physical_utilization", "dash.gb_available",
    "dash.system_volume_usage", "dash.gb_total", "dash.recoverable_space",
    // Memory engine
    "mem.title", "mem.subtitle", "mem.pressure_label", "mem.phys_utilization",
    "mem.hardware_badge", "mem.standby_cached", "mem.cached_ready", "mem.active_ram_load",
    "mem.available", "mem.cached", "mem.paged_pool", "mem.smart_optimization",
    "mem.smart_optimization_desc", "mem.trim_now", "mem.auto_trimmer", "mem.in_use_fmt",
    // Process manager
    "proc.title", "proc.subtitle", "proc.refresh", "proc.filter_placeholder",
    "proc.active_tasks", "proc.col_name", "proc.col_pid", "proc.col_working_set",
    "proc.col_security", "proc.col_action", "proc.badge_critical", "proc.badge_signed",
    "proc.badge_user", "proc.protected", "proc.end_task", "proc.no_match",
    "proc.terminated_toast",
    // Debloat
    "deb.title", "deb.subtitle", "deb.preset_label", "deb.safe_preset", "deb.balanced_preset",
    "deb.aggressive_preset", "deb.rules_suffix", "deb.apply_preset_btn",
    "deb.safe_banner_title", "deb.safe_banner_desc", "deb.balanced_banner_title",
    "deb.balanced_banner_desc", "deb.aggressive_banner_title", "deb.aggressive_banner_desc",
    "deb.confirm_required", "deb.confirm_body", "deb.confirm_apply_btn", "deb.cancel_safe_btn",
    "deb.preview_title", "deb.preview_desc", "deb.pending_changes", "deb.registry_keys",
    "deb.packages", "deb.services_label", "deb.no_pending", "deb.vss_note",
    // Cleaner
    "clean.title", "clean.subtitle", "clean.scan_storage", "clean.clean_now",
    "clean.files_ready", "clean.mb_size",
    // Context menu
    "ctx.title", "ctx.subtitle", "ctx.rescan_handlers", "ctx.scan_hint",
    "ctx.disable_btn", "ctx.enable_btn", "ctx.backup_note", "ctx.root_badge",
    "ctx.machine_store", "ctx.user_store", "ctx.disabled_badge", "ctx.active_badge",
    "ctx.none_found",
    // Network / DNS
    "net.title", "net.subtitle", "net.flush_cache", "net.flush_hint", "net.preset_label",
    "net.apply_all_adapters", "net.adapters", "net.current_servers", "net.auto_note",
    "net.admin_note", "net.adapter_count",
    // Scheduled tasks
    "tasks.title", "tasks.subtitle", "tasks.rescan", "tasks.hint", "tasks.disable_btn",
    "tasks.enable_btn", "tasks.snapshot_note", "tasks.safe_note", "tasks.none_found",
    "tasks.xml_locked",
    // Startup / Services / Privacy / Gaming / Health / Restore / Logs
    "su.title", "svc.title", "priv.title", "game.title", "health.title", "restore.title",
    "logs.title",
    // Settings
    "set.title", "set.subtitle", "set.appearance", "set.theme_label", "set.language_label",
    "set.language_hint", "set.privileges", "set.current_status", "set.restart_admin",
    "set.elevate_note", "set.auto_trim_section", "set.auto_trim_enable",
    "set.auto_trim_threshold", "set.auto_trim_cooldown", "set.about", "set.about_version",
    "set.license_line",
];

/// Translate `key` into `lang`. Falls back to English when a key has no
/// Indonesian entry (should never happen thanks to the parity test).
pub fn tr(lang: Lang, key: &str) -> &'static str {
    let english = tr_en(key);
    if lang == Lang::Id {
        tr_id(key).unwrap_or(english)
    } else {
        english
    }
}

fn tr_en(key: &str) -> &'static str {
    match key {
        // ---- Navigation ----
        "nav.dashboard" => "Dashboard",
        "nav.memory" => "Memory Engine",
        "nav.processes" => "Process Manager",
        "nav.debloat" => "Debloater",
        "nav.startup" => "Startup Apps",
        "nav.services" => "Service Manager",
        "nav.privacy" => "Privacy Center",
        "nav.cleaner" => "Storage Cleaner",
        "nav.gaming" => "Gaming Profile",
        "nav.health" => "Health Diagnostics",
        "nav.restore" => "Restore Points",
        "nav.contextmenu" => "Context Menu Cleaner",
        "nav.network" => "DNS & Network",
        "nav.tasks" => "Scheduled Tasks",
        "nav.logs" => "Audit Logs",
        "nav.settings" => "Settings",

        // ---- Common ----
        "common.apply" => "Apply",
        "common.rescan" => "Rescan",
        "common.scan_now" => "Scan Now",
        "common.cancel" => "Cancel",
        "common.dry_run_test" => "Dry-Run Test",
        "common.search" => "Search:",
        "common.filter_view" => "Filter View:",
        "common.enabled" => "Enabled",
        "common.disabled" => "Disabled",
        "common.loading" => "Scanning...",
        "common.benefit" => "Benefit:",
        "common.target" => "Target:",
        "common.all" => "All",
        "common.optimized" => "✓ Optimized",
        "common.admin_elevated" => "Admin • Elevated",
        "common.standard_user" => "Standard User",
        "common.scanning" => "Scanning",
        "common.status_ready" => "System Ready",
        "common.live_badge" => "LIVE",

        // ---- Dashboard ----
        "dash.title" => "System Overview",
        "dash.subtitle" => "Real-time telemetry, hardware metrics & automated health recommendations",
        "dash.refresh_telemetry" => "↻ Refresh Telemetry",
        "dash.cpu_telemetry" => "CPU Telemetry",
        "dash.host_info" => "Host Info",
        "dash.active_load" => "Active Load",
        "dash.cores_threads" => "Cores / Threads",
        "dash.memory_engine" => "Memory Engine",
        "dash.storage_status" => "Storage Status (C:)",
        "dash.quick_trim" => "⚡ Quick Memory Trim",
        "dash.inspect_engine" => "Inspect Engine →",
        "dash.scan_junk" => "Scan Junk Files",
        "dash.health_actions" => "System Health & Actions",
        "dash.protected_ok" => "0 Critical Alerts • Protected",
        "dash.attention_needed" => "1 Alert • Attention Required",
        "dash.av_active_title" => "Antivirus & Real-Time Protection Active",
        "dash.av_active_desc" => "Windows Defender real-time monitoring and firewall filters are operating normally.",
        "dash.av_off_title" => "Antivirus Protection Disabled",
        "dash.av_off_desc" => "Real-time protection is offline. System may be vulnerable to unauthorized execution.",
        "dash.open_health" => "Health Diagnostics →",
        "dash.physical_utilization" => "Physical Utilization",
        "dash.gb_available" => "GB Available",
        "dash.system_volume_usage" => "System Volume Usage",
        "dash.gb_total" => "GB Total",
        "dash.recoverable_space" => "Recoverable Disk Space",

        // ---- Memory engine ----
        "mem.title" => "Memory Engine",
        "mem.subtitle" => "Real-time physical RAM telemetry, working set optimization & committed memory breakdown",
        "mem.pressure_label" => "Pressure:",
        "mem.phys_utilization" => "Physical Memory Utilization",
        "mem.hardware_badge" => "HARDWARE",
        "mem.standby_cached" => "Standby Cached",
        "mem.cached_ready" => "Cached Ready",
        "mem.active_ram_load" => "Active RAM Load",
        "mem.available" => "AVAILABLE",
        "mem.cached" => "CACHED",
        "mem.paged_pool" => "PAGED POOL",
        "mem.smart_optimization" => "Smart Optimization",
        "mem.smart_optimization_desc" => "Trim background process working sets and reclaim inactive memory blocks.",
        "mem.trim_now" => "⚡ Optimize Memory Now",
        "mem.auto_trimmer" => "Auto Memory Trimmer",
        "mem.in_use_fmt" => "IN USE",

        // ---- Process manager ----
        "proc.title" => "Process Manager",
        "proc.subtitle" => "Inspect active processes, memory working sets, CPU usage, and signature trust",
        "proc.refresh" => "↻ Refresh Processes",
        "proc.filter_placeholder" => "Filter processes by name or PID...",
        "proc.active_tasks" => "active tasks",
        "proc.col_name" => "Process Name",
        "proc.col_pid" => "PID",
        "proc.col_working_set" => "Working Set",
        "proc.col_security" => "Security Status",
        "proc.col_action" => "Action",
        "proc.badge_critical" => "Critical System",
        "proc.badge_signed" => "Signed Trust",
        "proc.badge_user" => "User App",
        "proc.protected" => "Protected",
        "proc.end_task" => "End Task",
        "proc.no_match" => "No processes match your filter.",
        "proc.terminated_toast" => "Terminated process PID",

        // ---- Debloat ----
        "deb.title" => "Windows Debloater & Optimizer",
        "deb.subtitle" => "Remove preinstalled bloat and quiet noisy background services",
        "deb.preset_label" => "Optimization Preset:",
        "deb.safe_preset" => "Safe",
        "deb.balanced_preset" => "Balanced",
        "deb.aggressive_preset" => "Aggressive",
        "deb.rules_suffix" => "rules",
        "deb.apply_preset_btn" => "Apply Preset",
        "deb.safe_banner_title" => "Safe Preset: Recommended For All Users (Zero Risk)",
        "deb.safe_banner_desc" => "Removes promotional UWP app stubs (Candy Crush, Feedback Hub, Solitaire, Tips), disables Bing web queries in Start Menu, and disables advertising tracking ID. 100% reversible with zero feature breakage.",
        "deb.balanced_banner_title" => "Balanced Preset Attention: Workstation & Productivity Profile",
        "deb.balanced_banner_desc" => "Builds on Safe. It turns off Widgets, Copilot, Edge prelaunch, and idle Xbox services. Wino creates a VSS restore point before it changes anything.",
        "deb.aggressive_banner_title" => "Aggressive Preset Attention & Caution: Deep Debloat Profile",
        "deb.aggressive_banner_desc" => "Adds to Safe and Balanced. It removes OEM stubs such as TikTok and Spotify and turns off location and activity history. Wino creates a VSS restore point before it changes anything.",
        "deb.confirm_required" => "Confirmation Required:",
        "deb.confirm_body" => "Review the list below. Wino saves a config snapshot and a VSS restore point before it applies changes.",
        "deb.confirm_apply_btn" => "✓ Create Snapshot & Apply",
        "deb.cancel_safe_btn" => "✕ Cancel / Stay on Safe",
        "deb.preview_title" => "Dry-Run Preview: Pending Changes",
        "deb.preview_desc" => "Exactly what will be modified when you press Apply:",
        "deb.pending_changes" => "pending change(s)",
        "deb.registry_keys" => "Registry Keys",
        "deb.packages" => "AppX Packages (via DISM)",
        "deb.services_label" => "Services → Manual",
        "deb.no_pending" => "This preset shows no pending changes. You already applied it.",
        "deb.vss_note" => "🛡 Pre-flight VSS restore point enabled",

        // ---- Cleaner ----
        "clean.title" => "Storage Cleaner",
        "clean.subtitle" => "Safe removal of obsolete temporary files, caches, and crash logs",
        "clean.scan_storage" => "Scan Storage",
        "clean.clean_now" => "Clean Now",
        "clean.files_ready" => "total files ready for safe cleanup",
        "clean.mb_size" => "MB",

        // ---- Context menu ----
        "ctx.title" => "Context Menu Cleaner",
        "ctx.subtitle" => "Make right-click menus snap open. Turn off slow or dead shell handlers. You can undo any change.",
        "ctx.rescan_handlers" => "↻ Rescan Handlers",
        "ctx.scan_hint" => "You disable a handler when you prefix its CLSID with -. Wino saves the original value in a snapshot.",
        "ctx.disable_btn" => "Disable",
        "ctx.enable_btn" => "Enable",
        "ctx.backup_note" => "Each toggle saves a rollback snapshot.",
        "ctx.root_badge" => "Root",
        "ctx.machine_store" => "Machine",
        "ctx.user_store" => "User",
        "ctx.disabled_badge" => "Disabled",
        "ctx.active_badge" => "Active",
        "ctx.none_found" => "No third-party context menu handlers found.",

        // ---- Network / DNS ----
        "net.title" => "DNS Optimizer & Network Tools",
        "net.subtitle" => "Flush the resolver cache and switch DNS with one click",
        "net.flush_cache" => "⚡ Flush DNS Cache",
        "net.flush_hint" => "Calls DnsFlushResolverCache in dnsapi.dll. No subprocess spawns.",
        "net.preset_label" => "DNS Preset:",
        "net.apply_all_adapters" => "Apply to All Adapters",
        "net.adapters" => "Network Adapters",
        "net.current_servers" => "Current DNS:",
        "net.auto_note" => "\"Automatic\" removes static overrides so DHCP-assigned DNS takes over again.",
        "net.admin_note" => "Changing adapter DNS requires administrator privileges.",
        "net.adapter_count" => "adapter(s) detected",

        // ---- Scheduled tasks ----
        "tasks.title" => "Scheduled Tasks Debloater",
        "tasks.subtitle" => "Detect and disable hidden telemetry, CEIP, and third-party auto-updater tasks",
        "tasks.rescan" => "↻ Rescan Tasks",
        "tasks.hint" => "Wino toggles tasks with schtasks.exe. It backs up the XML before it changes anything.",
        "tasks.disable_btn" => "Disable Task",
        "tasks.enable_btn" => "Enable Task",
        "tasks.snapshot_note" => "A rollback snapshot is recorded before each disable action.",
        "tasks.safe_note" => "You can disable the tasks below. Windows brings them back after a major feature update when it needs them.",
        "tasks.none_found" => "No known telemetry or updater tasks found on this system.",
        "tasks.xml_locked" => "XML locked",

        // ---- Other tabs ----
        "su.title" => "Startup Applications",
        "svc.title" => "Service Manager",
        "priv.title" => "Privacy Center",
        "game.title" => "Gaming Profile",
        "health.title" => "Health Diagnostics",
        "restore.title" => "Restore Points & Snapshots",
        "logs.title" => "Audit Logs",

        // ---- Settings ----
        "set.title" => "Settings & Preferences",
        "set.subtitle" => "Configure application behavior, language, themes, and privileges",
        "set.appearance" => "Appearance & Language",
        "set.theme_label" => "Theme:",
        "set.language_label" => "Language / Bahasa:",
        "set.language_hint" => "You see the new language on every panel. Technical names stay in English.",
        "set.privileges" => "Execution Privileges",
        "set.current_status" => "Current Status:",
        "set.restart_admin" => "Restart as Administrator",
        "set.elevate_note" => "Some deep registry modifications and system package removals require administrator privileges.",
        "set.auto_trim_section" => "Auto Memory Trimmer (Background Mode)",
        "set.auto_trim_enable" => "Enable power-friendly automatic trim when RAM usage exceeds threshold",
        "set.auto_trim_threshold" => "RAM Threshold:",
        "set.auto_trim_cooldown" => "Cooldown between automatic trims protects responsiveness.",
        "set.about" => "About Wino",
        "set.about_version" => "Version: 2.5.0 | Pure Rust Implementation | Safe, Reversible & Zero-Placebo",
        "set.license_line" => "License: MIT / Apache-2.0",

        _ => "",
    }
}

fn tr_id(key: &str) -> Option<&'static str> {
    Some(match key {
        // ---- Navigasi ----
        "nav.dashboard" => "Dasbor",
        "nav.memory" => "Mesin Memori",
        "nav.processes" => "Manajer Proses",
        "nav.debloat" => "Debloater",
        "nav.startup" => "Aplikasi Startup",
        "nav.services" => "Manajer Layanan",
        "nav.privacy" => "Pusat Privasi",
        "nav.cleaner" => "Pembersih Penyimpanan",
        "nav.gaming" => "Profil Gaming",
        "nav.health" => "Diagnostik Kesehatan",
        "nav.restore" => "Titik Pemulihan",
        "nav.contextmenu" => "Pembersih Menu Klik Kanan",
        "nav.network" => "DNS & Jaringan",
        "nav.tasks" => "Terdjadwal (Tasks)",
        "nav.logs" => "Log Audit",
        "nav.settings" => "Pengaturan",

        // ---- Umum ----
        "common.apply" => "Terapkan",
        "common.rescan" => "Pindai Ulang",
        "common.scan_now" => "Pindai Sekarang",
        "common.cancel" => "Batal",
        "common.dry_run_test" => "Uji Dry-Run",
        "common.search" => "Cari:",
        "common.filter_view" => "Filter Tampilan:",
        "common.enabled" => "Aktif",
        "common.disabled" => "Nonaktif",
        "common.loading" => "Memindai...",
        "common.benefit" => "Manfaat:",
        "common.target" => "Target:",
        "common.all" => "Semua",
        "common.optimized" => "✓ Teroptimasi",
        "common.admin_elevated" => "Admin • Elevated",
        "common.standard_user" => "Pengguna Standar",
        "common.scanning" => "Memindai",
        "common.status_ready" => "Sistem Siap",
        "common.live_badge" => "LANGSUNG",

        // ---- Dasbor ----
        "dash.title" => "Ikhtisar Sistem",
        "dash.subtitle" => "Telemetri real-time, metrik perangkat keras & rekomendasi kesehatan otomatis",
        "dash.refresh_telemetry" => "↻ Muat Ulang Telemetri",
        "dash.cpu_telemetry" => "Telemetri CPU",
        "dash.host_info" => "Info Host",
        "dash.active_load" => "Beban Aktif",
        "dash.cores_threads" => "Inti / Thread",
        "dash.memory_engine" => "Mesin Memori",
        "dash.storage_status" => "Status Penyimpanan (C:)",
        "dash.quick_trim" => "⚡ Trim Memori Cepat",
        "dash.inspect_engine" => "Buka Mesin →",
        "dash.scan_junk" => "Pindai File Sampah",
        "dash.health_actions" => "Kesehatan Sistem & Aksi",
        "dash.protected_ok" => "0 Peringatan Kritis • Terlindungi",
        "dash.attention_needed" => "1 Peringatan • Perlu Perhatian",
        "dash.av_active_title" => "Antivirus & Perlindungan Real-Time Aktif",
        "dash.av_active_desc" => "Monitoring real-time Windows Defender dan filter firewall berjalan normal.",
        "dash.av_off_title" => "Perlindungan Antivirus Nonaktif",
        "dash.av_off_desc" => "Perlindungan real-time sedang offline. Sistem rentan terhadap eksekusi tidak sah.",
        "dash.open_health" => "Diagnostik Kesehatan →",
        "dash.physical_utilization" => "Utilisasi Fisik",
        "dash.gb_available" => "GB Tersedia",
        "dash.system_volume_usage" => "Penggunaan Volume Sistem",
        "dash.gb_total" => "Total GB",
        "dash.recoverable_space" => "Ruang Disk yang Dapat Dipulihkan",

        // ---- Mesin memori ----
        "mem.title" => "Mesin Memori",
        "mem.subtitle" => "Telemetri RAM fisik real-time, optimasi working set & rincian memori commit",
        "mem.pressure_label" => "Tekanan:",
        "mem.phys_utilization" => "Utilisasi Memori Fisik",
        "mem.hardware_badge" => "PERANGKAT",
        "mem.standby_cached" => "Cache Standby",
        "mem.cached_ready" => "Cache Siap Pakai",
        "mem.active_ram_load" => "Beban RAM Aktif",
        "mem.available" => "TERSEDIA",
        "mem.cached" => "CACHE",
        "mem.paged_pool" => "PAGED POOL",
        "mem.smart_optimization" => "Optimasi Cerdas",
        "mem.smart_optimization_desc" => "Trim working set proses latar belakang dan klaim blok memori tidak aktif.",
        "mem.trim_now" => "⚡ Optimalkan Memori Sekarang",
        "mem.auto_trimmer" => "Pemangkas Memori Otomatis",
        "mem.in_use_fmt" => "TERPAKAI",

        // ---- Manajer proses ----
        "proc.title" => "Manajer Proses",
        "proc.subtitle" => "Pantau proses aktif, working set memori, penggunaan CPU, dan tanda tangan keamanan",
        "proc.refresh" => "↻ Muat Ulang Proses",
        "proc.filter_placeholder" => "Filter proses berdasarkan nama atau PID...",
        "proc.active_tasks" => "tugas aktif",
        "proc.col_name" => "Nama Proses",
        "proc.col_pid" => "PID",
        "proc.col_working_set" => "Working Set",
        "proc.col_security" => "Status Keamanan",
        "proc.col_action" => "Aksi",
        "proc.badge_critical" => "Sistem Kritis",
        "proc.badge_signed" => "Terverifikasi",
        "proc.badge_user" => "Aplikasi Pengguna",
        "proc.protected" => "Dilindungi",
        "proc.end_task" => "Akhiri Tugas",
        "proc.no_match" => "Tidak ada proses yang cocok dengan filter.",
        "proc.terminated_toast" => "Menghentikan proses PID",

        // ---- Debloat ----
        "deb.title" => "Windows Debloater & Optimizer",
        "deb.subtitle" => "Hapus bawaan pabrik dan tenangkan layanan latar belakang yang berisik",
        "deb.preset_label" => "Preset Optimasi:",
        "deb.safe_preset" => "Aman",
        "deb.balanced_preset" => "Seimbang",
        "deb.aggressive_preset" => "Agresif",
        "deb.rules_suffix" => "aturan",
        "deb.apply_preset_btn" => "Terapkan Preset",
        "deb.safe_banner_title" => "Preset Aman: Direkomendasikan untuk Semua Pengguna (Nol Risiko)",
        "deb.safe_banner_desc" => "Menghapus aplikasi UWP promosi (Candy Crush, Feedback Hub, Solitaire, Tips), mematikan pencarian Bing di Start Menu, dan menonaktifkan advertising ID. 100% dapat dikembalikan tanpa merusak fitur.",
        "deb.balanced_banner_title" => "Perhatian Preset Seimbang: Profil Workstation & Produktivitas",
        "deb.balanced_banner_desc" => "Melengkapi preset Aman. Fitur ini mematikan Widgets, Copilot, prelaunch Edge, dan layanan Xbox yang menganggur. Wino membuat titik pemulihan VSS sebelum mengubah apa pun.",
        "deb.aggressive_banner_title" => "Perhatian & Kehati-hatian Preset Agresif: Profil Debloat Mendalam",
        "deb.aggressive_banner_desc" => "Menambah aturan Aman dan Seimbang. Fitur ini menghapus stub OEM seperti TikTok dan Spotify dan mematikan sensor lokasi dan riwayat aktivitas. Wino membuat titik pemulihan VSS sebelum mengubah apa pun.",
        "deb.confirm_required" => "Konfirmasi Diperlukan:",
        "deb.confirm_body" => "Tinjau daftar di bawah ini. Wino menyimpan snapshot konfigurasi dan titik pemulihan VSS sebelum menerapkan perubahan.",
        "deb.confirm_apply_btn" => "✓ Buat Snapshot & Terapkan",
        "deb.cancel_safe_btn" => "✕ Batal / Tetap di Aman",
        "deb.preview_title" => "Pratinjau Dry-Run: Perubahan yang Akan Dilakukan",
        "deb.preview_desc" => "Persisnya apa yang akan dimodifikasi saat Anda menekan Terapkan:",
        "deb.pending_changes" => "perubahan tertunda",
        "deb.registry_keys" => "Kunci Registri",
        "deb.packages" => "Paket AppX (via DISM)",
        "deb.services_label" => "Layanan → Manual",
        "deb.no_pending" => "Preset ini tidak punya perubahan tertunda. Anda sudah menerapkannya.",
        "deb.vss_note" => "🛡 Titik pemulihan VSS pra-penerbangan aktif",

        // ---- Pembersih penyimpanan ----
        "clean.title" => "Pembersih Penyimpanan",
        "clean.subtitle" => "Penghapusan aman file sementara usang, cache, dan log crash",
        "clean.scan_storage" => "Pindai Penyimpanan",
        "clean.clean_now" => "Bersihkan Sekarang",
        "clean.files_ready" => "total file siap dibersihkan dengan aman",
        "clean.mb_size" => "MB",

        // ---- Menu klik kanan ----
        "ctx.title" => "Pembersih Menu Klik Kanan",
        "ctx.subtitle" => "Buat menu klik kanan membuka lebih cepat. Nonaktifkan handler yang lambat atau mati. Anda bisa membatalkan perubahan kapan saja.",
        "ctx.rescan_handlers" => "↻ Pindai Ulang Handler",
        "ctx.scan_hint" => "Anda menonaktifkan handler dengan memberi awalan '-' pada CLSID. Wino menyimpan nilai asli di snapshot.",
        "ctx.disable_btn" => "Nonaktifkan",
        "ctx.enable_btn" => "Aktifkan",
        "ctx.backup_note" => "Sakelar ini menyimpan snapshot rollback.",
        "ctx.root_badge" => "Root",
        "ctx.machine_store" => "Mesin",
        "ctx.user_store" => "Pengguna",
        "ctx.disabled_badge" => "Nonaktif",
        "ctx.active_badge" => "Aktif",
        "ctx.none_found" => "Tidak ada handler menu klik kanan pihak ketiga yang ditemukan.",

        // ---- Jaringan / DNS ----
        "net.title" => "Optimizer DNS & Alat Jaringan",
        "net.subtitle" => "Bersihkan cache resolver dan ganti DNS dalam satu klik",
        "net.flush_cache" => "⚡ Bersihkan Cache DNS",
        "net.flush_hint" => "Memanggil DnsFlushResolverCache di dnsapi.dll. Tanpa subprocess.",
        "net.preset_label" => "Preset DNS:",
        "net.apply_all_adapters" => "Terapkan ke Semua Adapter",
        "net.adapters" => "Adapter Jaringan",
        "net.current_servers" => "DNS Saat Ini:",
        "net.auto_note" => "Pilihan \"Otomatis\" menghapus override statis sehingga DNS dari DHCP dipakai kembali.",
        "net.admin_note" => "Mengubah DNS adapter membutuhkan hak administrator.",
        "net.adapter_count" => "adapter terdeteksi",

        // ---- Task terjadwal ----
        "tasks.title" => "Pembersih Task Terjadwal",
        "tasks.subtitle" => "Deteksi dan nonaktifkan telemetri tersembunyi, CEIP, serta auto-updater pihak ketiga",
        "tasks.rescan" => "↻ Pindai Ulang Task",
        "tasks.hint" => "Wino mengaktifkan atau menonaktifkan task lewat schtasks.exe. Ia mencadangkan XML sebelum mengubah apa pun.",
        "tasks.disable_btn" => "Nonaktifkan Task",
        "tasks.enable_btn" => "Aktifkan Task",
        "tasks.snapshot_note" => "Snapshot rollback dicatat sebelum setiap aksi penonaktifan.",
        "tasks.safe_note" => "Anda bisa menonaktifkan task di bawah ini. Windows akan mengaktifkannya lagi setelah update fitur besar bila perlu.",
        "tasks.none_found" => "Tidak ada task telemetri atau updater yang dikenali di sistem ini.",
        "tasks.xml_locked" => "XML terkunci",

        // ---- Tab lainnya ----
        "su.title" => "Aplikasi Startup",
        "svc.title" => "Manajer Layanan",
        "priv.title" => "Pusat Privasi",
        "game.title" => "Profil Gaming",
        "health.title" => "Diagnostik Kesehatan",
        "restore.title" => "Titik Pemulihan & Snapshot",
        "logs.title" => "Log Audit",

        // ---- Pengaturan ----
        "set.title" => "Pengaturan & Preferensi",
        "set.subtitle" => "Atur perilaku aplikasi, bahasa, tema, dan privilese",
        "set.appearance" => "Tampilan & Bahasa",
        "set.theme_label" => "Tema:",
        "set.language_label" => "Bahasa / Language:",
        "set.language_hint" => "Anda melihat bahasa baru di setiap panel. Nama teknis tetap dalam bahasa Inggris.",
        "set.privileges" => "Privilese Eksekusi",
        "set.current_status" => "Status Saat Ini:",
        "set.restart_admin" => "Jalankan Ulang sebagai Administrator",
        "set.elevate_note" => "Beberapa modifikasi registri mendalam dan penghapusan paket sistem membutuhkan hak administrator.",
        "set.auto_trim_section" => "Pemangkas Memori Otomatis (Mode Latar Belakang)",
        "set.auto_trim_enable" => "Aktifkan trim otomatis hemat daya saat penggunaan RAM melampaui batas",
        "set.auto_trim_threshold" => "Ambang RAM:",
        "set.auto_trim_cooldown" => "Jeda antar trim otomatis menjaga responsivitas sistem.",
        "set.about" => "Tentang Wino",
        "set.about_version" => "Versi: 2.5.0 | Implementasi Rust Murni | Aman, Dapat Dikembalikan & Nol-Placebo",
        "set.license_line" => "Lisensi: MIT / Apache-2.0",

        _ => return None,
    })
}







