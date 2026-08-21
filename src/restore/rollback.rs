use crate::core::executor::SystemExecutor;
use crate::core::logger::log_info;
use crate::restore::snapshots::list_snapshots;

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

    let msg = format!("Successfully restored {} settings from snapshot '{}'.", restored_count, snapshot.description);
    log_info("restore", &msg);
    Ok(msg)
}
