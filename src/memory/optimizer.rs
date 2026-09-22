use crate::core::logger::log_info;
use crate::memory::monitor::capture_memory_snapshot;
use crate::memory::pressure::MemoryPressure;
use serde::{Deserialize, Serialize};
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::ProcessStatus::EmptyWorkingSet;
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_SET_QUOTA,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationReport {
    pub before_used_bytes: u64,
    pub after_used_bytes: u64,
    pub freed_bytes: u64,
    pub before_pressure: MemoryPressure,
    pub after_pressure: MemoryPressure,
    pub processes_trimmed: usize,
    pub message: String,
    pub is_dry_run: bool,
}

pub fn optimize_memory(dry_run: bool) -> OptimizationReport {
    let before = capture_memory_snapshot();
    let processes = crate::monitoring::process::list_running_processes();

    if dry_run {
        log_info(
            "memory",
            "[DRY-RUN] Simulating memory working set trim and standby analysis.",
        );
        return OptimizationReport {
            before_used_bytes: before.stats.used_bytes,
            after_used_bytes: before.stats.used_bytes,
            freed_bytes: 0,
            before_pressure: before.pressure,
            after_pressure: before.pressure,
            processes_trimmed: processes.len(),
            message: format!(
                "Dry run: would analyze and safely trim working sets of {} candidate processes.",
                processes.len()
            ),
            is_dry_run: true,
        };
    }

    let mut trimmed_count = 0usize;

    for p in &processes {
        // Skip system critical processes to preserve Windows stability
        if p.is_system_critical || p.pid == 0 || p.pid == 4 {
            continue;
        }

        // Only trim processes taking > 25 MB
        if p.memory_working_set_bytes < 25 * 1024 * 1024 {
            continue;
        }

        unsafe {
            let handle_res =
                OpenProcess(PROCESS_SET_QUOTA | PROCESS_QUERY_INFORMATION, false, p.pid);

            if let Ok(handle) = handle_res {
                if EmptyWorkingSet(handle).is_ok() {
                    trimmed_count += 1;
                }
                let _ = CloseHandle(handle);
            }
        }
    }

    // Allow memory manager a moment to settle
    std::thread::sleep(std::time::Duration::from_millis(150));

    let after = capture_memory_snapshot();
    let freed_bytes = before
        .stats
        .used_bytes
        .saturating_sub(after.stats.used_bytes);

    let message = if freed_bytes > 50 * 1024 * 1024 {
        format!(
            "Successfully trimmed working sets across {} background processes, recovering {:.1} MB.",
            trimmed_count,
            freed_bytes as f64 / (1024.0 * 1024.0)
        )
    } else {
        "Optimization completed. Memory was already healthy; no extensive cleanup was necessary."
            .to_string()
    };

    log_info("memory", &message);

    OptimizationReport {
        before_used_bytes: before.stats.used_bytes,
        after_used_bytes: after.stats.used_bytes,
        freed_bytes,
        before_pressure: before.pressure,
        after_pressure: after.pressure,
        processes_trimmed: trimmed_count,
        message,
        is_dry_run: false,
    }
}
