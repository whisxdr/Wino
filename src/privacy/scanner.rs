use crate::core::executor::{ExecutionResult, SystemExecutor};
use crate::privacy::policies::{load_privacy_rules, PrivacyRule};
use crate::restore::snapshots::create_snapshot;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedPrivacyItem {
    pub rule: PrivacyRule,
    pub is_applied: bool,
}

pub fn scan_privacy_items() -> Vec<ScannedPrivacyItem> {
    let rules = load_privacy_rules();
    let mut items = Vec::new();

    for rule in rules {
        let mut all_applied = true;
        for reg in &rule.registry_keys {
            let cur = SystemExecutor::read_registry_dword(&reg.hive, &reg.path, &reg.value_name);
            if cur != Some(reg.value_data) {
                all_applied = false;
                break;
            }
        }

        items.push(ScannedPrivacyItem {
            rule,
            is_applied: all_applied,
        });
    }

    items
}

pub fn apply_privacy_rule(
    rule: &PrivacyRule,
    enable_protection: bool,
    dry_run: bool,
) -> Vec<ExecutionResult> {
    if !dry_run {
        let _ = create_snapshot(&format!("Privacy rule: {}", rule.name));
    }

    let mut results = Vec::new();

    for reg in &rule.registry_keys {
        let target_val = if enable_protection {
            reg.value_data
        } else {
            reg.restore_data
        };

        let res = SystemExecutor::set_registry_dword(
            &reg.hive,
            &reg.path,
            &reg.value_name,
            target_val,
            dry_run,
        );
        results.push(res);
    }

    results
}
