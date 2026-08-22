use crate::core::executor::SystemExecutor;
use crate::core::logger::{log_info, log_warn};
use crate::core::regutil;
use crate::restore::snapshots::list_snapshots;

fn hive_from_label(label: &str) -> windows::Win32::System::Registry::HKEY {
    use windows::Win32::System::Registry::{HKEY_CLASSES_ROOT, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    match label.to_uppercase().as_str() {
        "HKCU" | "HKEY_CURRENT_USER" => HKEY_CURRENT_USER,
        "HKCR" | "HKEY_CLASSES_ROOT" => HKEY_CLASSES_ROOT,
        _ => HKEY_LOCAL_MACHINE,
    }
}

pub fn rollback_snapshot(snapshot_id: &str) -> Result<String, String> {
    let snapshots = list_snapshots();
    let Some(snapshot) = snapshots.iter().find(|s| s.id == snapshot_id) else {
        return Err(format!("Snapshot ID '{}' not found.", snapshot_id));
    };

    let mut restored_count = 0usize;

    for reg in &snapshot.registry_entries {
        if let Some(prev) = reg.previous_value {
            let res = SystemExecutor::set_registry_dword(&reg.hive, &reg.path, &reg.value_name, prev, false);
            if res.success {
                restored_count += 1;
            }
        } else {
            let res = SystemExecutor::delete_registry_value(&reg.hive, &reg.path, &reg.value_name, false);
            if res.success {
                restored_count += 1;
            }
        }
    }

    // Restore REG_SZ values (context-menu handlers, DNS server overrides, ...)
    for entry in &snapshot.string_entries {
        let hive = hive_from_label(&entry.hive);
        let result = match &entry.previous_value {
            Some(previous) => regutil::write_string(hive, &entry.path, &entry.value_name, previous),
            None => regutil::delete_value(hive, &entry.path, &entry.value_name),
        };
        if result.is_ok() {
            restored_count += 1;
        } else {
            log_warn("restore", &format!("Failed to restore string value '{}\\{}\\{}'", entry.hive, entry.path, entry.value_name));
        }
    }

    // Re-enable scheduled tasks that were disabled while this snapshot was taken.
    for task in &snapshot.task_entries {
        if task.previous_enabled {
            if let Err(e) = crate::tasks::manager::set_task_enabled(&task.task_path, true, false) {
                log_warn("restore", &format!("Failed to re-enable task '{}': {}", task.task_path, e));
            } else {
                restored_count += 1;
            }
        }
    }

    let msg = format!("Successfully restored {} settings from snapshot '{}'.", restored_count, snapshot.description);
    log_info("restore", &msg);
    Ok(msg)
}
