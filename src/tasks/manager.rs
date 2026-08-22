use std::os::windows::process::CommandExt;
use std::process::Command;

use crate::core::logger::{log_error, log_info};
use crate::restore::snapshots::{create_snapshot_for_tasks, TaskBackupEntry};
use crate::tasks::scanner::tasks_root;

const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Enable or disable a scheduled task via the native Task Scheduler CLI
/// (schtasks.exe — an instant-spawn C/C++ System32 binary; zero PowerShell).
pub fn set_task_enabled(task_path: &str, enable: bool, dry_run: bool) -> Result<(), String> {
    let verb = if enable { "/ENABLE" } else { "/DISABLE" };
    let action = format!("{} task {}", verb.trim_start_matches('/'), task_path);

    if dry_run {
        log_info("tasks", &format!("[DRY-RUN] Would {}", action));
        return Ok(());
    }

    if !enable {
        // Snapshot the previous state so rollback can re-enable the task.
        let entry = TaskBackupEntry {
            task_path: task_path.to_string(),
            previous_enabled: true,
        };
        create_snapshot_for_tasks(&format!("Disable scheduled task {}", task_path), &[entry]);
    }

    let output = Command::new("schtasks.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["/Change", "/TN", task_path, verb])
        .output()
        .map_err(|e| format!("Failed to spawn schtasks.exe: {}", e))?;

    if output.status.success() {
        log_info("tasks", &action);
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let msg = format!("schtasks failed for '{}' (exit {:?}): {}",
            task_path,
            output.status.code(),
            if stderr.is_empty() { "access denied or task not found".to_string() } else { stderr });
        log_error("tasks", &msg);
        Err(msg)
    }
}

/// Verify a task path exists on disk in the task store.
pub fn task_exists_on_disk(task_path: &str) -> bool {
    let Some(root) = tasks_root() else { return false };
    let rel = task_path.trim_start_matches('\\');
    root.join(rel).is_file()
}
