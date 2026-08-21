use crate::core::executor::SystemExecutor;
use crate::core::logger::log_info;
use crate::restore::snapshots::create_snapshot;
use crate::startup::scanner::StartupItem;
use std::fs;
use std::path::Path;

pub fn toggle_startup_item(item: &StartupItem, enable: bool, dry_run: bool) -> Result<(), String> {
    if !dry_run {
        let _ = create_snapshot(&format!("Startup toggle: {}", item.name));
    }

    if dry_run {
        log_info("startup", &format!("[DRY-RUN] Would {} startup item: {}", if enable { "enable" } else { "disable" }, item.name));
        return Ok(());
    }

    if item.source.contains("HKCU") {
        if !enable {
            let res = SystemExecutor::delete_registry_value("HKCU", "Software\\Microsoft\\Windows\\CurrentVersion\\Run", &item.name, false);
            if res.success { Ok(()) } else { Err(res.details) }
        } else {
            // Re-enabling would re-write value if command exists
            Ok(())
        }
    } else if item.source.contains("HKLM") {
        if !enable {
            let res = SystemExecutor::delete_registry_value("HKLM", "Software\\Microsoft\\Windows\\CurrentVersion\\Run", &item.name, false);
            if res.success { Ok(()) } else { Err(res.details) }
        } else {
            Ok(())
        }
    } else if item.source.contains("Startup Folder") {
        let path = Path::new(&item.command);
        if path.exists() {
            if !enable {
                let disabled_path = path.with_extension("lnk.disabled");
                fs::rename(path, disabled_path).map_err(|e| e.to_string())?;
            } else {
                let enabled_path = path.with_extension("lnk");
                fs::rename(path, enabled_path).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    } else {
        Err("Unsupported startup source.".to_string())
    }
}
