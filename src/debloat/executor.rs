use crate::core::executor::{ExecutionResult, SystemExecutor};
use crate::core::logger::log_info;
use crate::debloat::rules::DebloatRule;
use crate::debloat::scanner::scan_debloat_items;
use crate::restore::snapshots::create_snapshot;

pub fn apply_debloat_rule(rule: &DebloatRule, dry_run: bool) -> Vec<ExecutionResult> {
    let mut results = Vec::new();

    // 1. Create a restore snapshot if not dry run
    if !dry_run {
        let _ = create_snapshot(&format!("Debloat: {}", rule.name));
    }

    // 2. Apply registry changes
    for reg in &rule.registry_keys {
        let res = SystemExecutor::set_registry_dword(
            &reg.hive,
            &reg.path,
            &reg.value_name,
            reg.value_data,
            dry_run,
        );
        results.push(res);
    }

    // 3. Remove packages if any
    for pkg in &rule.package_names {
        if dry_run {
            results.push(ExecutionResult {
                success: true,
                dry_run: true,
                action: format!("Remove AppX package: {}", pkg),
                details: "Would remove UWP app package.".to_string(),
            });
        } else {
            let res = crate::debloat::packages::remove_appx_package(pkg, dry_run);
            results.push(ExecutionResult {
                success: res.is_ok(),
                dry_run: false,
                action: format!("Remove AppX package: {}", pkg),
                details: res.err().unwrap_or_else(|| "Package removed.".to_string()),
            });
        }
    }

    // 4. Disable services if any
    for svc in &rule.services {
        if dry_run {
            results.push(ExecutionResult {
                success: true,
                dry_run: true,
                action: format!("Set service {} startup to Manual", svc),
                details: "Would configure service startup mode.".to_string(),
            });
        } else {
            let res = crate::services::manager::set_service_startup(svc, crate::services::manager::StartupType::Manual);
            results.push(ExecutionResult {
                success: res.is_ok(),
                dry_run: false,
                action: format!("Set service {} startup to Manual", svc),
                details: res.err().unwrap_or_else(|| "Service startup mode set to Manual.".to_string()),
            });
        }
    }

    results
}

pub fn is_rule_in_preset(rule_preset: &str, target_preset: &str) -> bool {
    match target_preset {
        "Safe" => rule_preset == "Safe",
        "Balanced" => rule_preset == "Safe" || rule_preset == "Balanced",
        "Aggressive" => rule_preset == "Safe" || rule_preset == "Balanced" || rule_preset == "Aggressive",
        _ => true,
    }
}

pub fn apply_debloat_preset(preset: &str, dry_run: bool) -> Vec<ExecutionResult> {
    let scanned = scan_debloat_items();
    let mut all_results = Vec::new();

    log_info("debloat", &format!("Applying debloat preset: '{}' (dry_run: {})", preset, dry_run));

    for item in scanned {
        if item.is_applied {
            continue;
        }

        if is_rule_in_preset(&item.rule.preset, preset) {
            let res = apply_debloat_rule(&item.rule, dry_run);
            all_results.extend(res);
        }
    }

    all_results
}
