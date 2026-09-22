use crate::core::executor::SystemExecutor;
use crate::core::system::SystemInfo;
use crate::debloat::rules::{load_default_rules, DebloatRule};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedDebloatItem {
    pub rule: DebloatRule,
    pub is_applied: bool,
    pub is_eligible: bool,
    pub status_text: String,
}

pub fn scan_debloat_items() -> Vec<ScannedDebloatItem> {
    let rules = load_default_rules();
    let sys_info = SystemInfo::detect();
    let mut scanned = Vec::new();

    for rule in rules {
        let is_version_supported = if rule.supported_windows.is_empty() {
            true
        } else {
            let cur_ver = if sys_info.is_windows_11 { "11" } else { "10" };
            rule.supported_windows.iter().any(|v| v == cur_ver)
        };

        if !is_version_supported {
            continue;
        }

        // Check if registry keys are already set
        let mut all_reg_applied = true;
        let mut has_reg = false;

        for reg in &rule.registry_keys {
            has_reg = true;
            let current =
                SystemExecutor::read_registry_dword(&reg.hive, &reg.path, &reg.value_name);
            if current != Some(reg.value_data) {
                all_reg_applied = false;
                break;
            }
        }

        let is_applied = if has_reg { all_reg_applied } else { false };

        let status_text = if is_applied {
            "Optimized / Inactive".to_string()
        } else {
            "Active / Enabled".to_string()
        };

        scanned.push(ScannedDebloatItem {
            rule,
            is_applied,
            is_eligible: !is_applied,
            status_text,
        });
    }

    scanned
}
