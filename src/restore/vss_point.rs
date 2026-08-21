use crate::core::logger::{log_info, log_warn};
use std::os::windows::process::CommandExt;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn create_windows_restore_point(description: &str) -> Result<String, String> {
    log_info("restore", &format!("Attempting to create Windows System Restore Point: '{}'", description));

    let output = Command::new("powershell.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &format!(
                "Checkpoint-Computer -Description 'Wino: {}' -RestorePointType 'MODIFY_SETTINGS' -ErrorAction Stop",
                description
            ),
        ])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let msg = format!("Windows System Restore Point '{}' created successfully.", description);
            log_info("restore", &msg);
            Ok(msg)
        }
        Ok(out) => {
            let err_str = String::from_utf8_lossy(&out.stderr).trim().to_string();
            let msg = format!("System Restore Point unavailable (System Protection may be disabled): {}", err_str);
            log_warn("restore", &msg);
            Err(msg)
        }
        Err(e) => {
            let msg = format!("Failed to invoke Checkpoint-Computer: {}", e);
            log_warn("restore", &msg);
            Err(msg)
        }
    }
}
