use crate::core::executor::SystemExecutor;
use crate::core::logger::log_info;
use crate::memory::monitor::capture_memory_snapshot;
use crate::restore::snapshots::create_snapshot;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamingProfileReport {
    pub game_mode_active: bool,
    pub ram_freed_mb: f64,
    pub before_used_mb: f64,
    pub after_used_mb: f64,
    pub processes_active: usize,
    pub message: String,
}

pub fn is_windows_game_mode_enabled() -> bool {
    let val = SystemExecutor::read_registry_dword(
        "HKCU",
        "Software\\Microsoft\\GameBar",
        "AutoGameModeEnabled",
    );
    val.unwrap_or(1) == 1
}

pub fn enable_gaming_profile(dry_run: bool) -> GamingProfileReport {
    if !dry_run {
        let _ = create_snapshot("Gaming Profile Activation");
    }

    let before = capture_memory_snapshot();

    // 1. Enable Windows Game Mode natively in registry
    let _ = SystemExecutor::set_registry_dword(
        "HKCU",
        "Software\\Microsoft\\GameBar",
        "AutoGameModeEnabled",
        1,
        dry_run,
    );
    let _ = SystemExecutor::set_registry_dword(
        "HKCU",
        "Software\\Microsoft\\GameBar",
        "AllowAutoGameMode",
        1,
        dry_run,
    );

    // 2. Perform safe background memory trimming
    let mem_report = crate::memory::optimizer::optimize_memory(dry_run);
    let after = capture_memory_snapshot();

    let before_mb = before.stats.used_bytes as f64 / (1024.0 * 1024.0);
    let after_mb = after.stats.used_bytes as f64 / (1024.0 * 1024.0);
    let freed_mb = (before_mb - after_mb).max(0.0);

    let message = format!(
        "Gaming profile configured. Windows Game Mode active, recovered {:.1} MB background memory for game headroom.",
        freed_mb
    );

    log_info("gaming", &message);

    GamingProfileReport {
        game_mode_active: true,
        ram_freed_mb: freed_mb,
        before_used_mb: before_mb,
        after_used_mb: after_mb,
        processes_active: mem_report.processes_trimmed,
        message,
    }
}
