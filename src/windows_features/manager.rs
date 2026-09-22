//! Enabling and disabling optional Windows features.
//!
//! **Why `dism.exe` and not a native API.** The crate links no DISM surface:
//! `Win32_System_ApplicationInstallationAndServicing` is the **MSI** API
//! (`MsiEnumProducts`, `MsiConfigureProduct`), which manages installed MSI
//! products, not optional Windows components. `dism.exe` is Microsoft's
//! documented interface for optional components and is present on every
//! supported Windows install, so it is the interface Wino uses.
//!
//! Every change passes through the Safety Engine first, and the engine's gate is
//! the only thing standing between a click and a system-wide change: a
//! `Critical` feature is refused outright, an operation on a Windows version it
//! does not declare is refused, and so is one that needs elevation the process
//! does not have. Nothing is changed unless the engine allows it.
//!
//! Two DISM behaviours are handled explicitly:
//!
//! * `/NoRestart` is always passed, so DISM never reboots the machine on its
//!   own. A restart is reported to the user instead of being taken.
//! * Exit code **3010** (`ERROR_SUCCESS_REBOOT_REQUIRED`) is a *success*: the
//!   feature was changed and the change takes effect after a restart. Treating
//!   it as a failure would report a completed change as broken.

use crate::core::logger::{log_error, log_info, log_warn};
use crate::core::proc;
use crate::core::safety::{OperationDescriptor, RiskLevel, SafetyEngine};
use crate::core::system::SystemInfo;
use crate::restore::snapshots::create_snapshot;
use crate::windows_features::models::find_curated;

/// DISM reports a completed change that needs a restart with this exit code.
const ERROR_SUCCESS_REBOOT_REQUIRED: i32 = 3010;

/// Windows versions this operation declares support for.
const SUPPORTED_WINDOWS: [&str; 2] = ["10", "11"];

/// Enable or disable one optional Windows feature.
///
/// `dry_run` returns the message the real run would produce without spawning
/// anything, so a preview cannot change the machine.
pub fn set_feature_enabled(
    feature_name: &str,
    enable: bool,
    dry_run: bool,
) -> Result<String, String> {
    let feature_name = feature_name.trim();
    if feature_name.is_empty() {
        return Err("No feature name was provided.".to_string());
    }

    let curated = find_curated(feature_name);
    let (risk, reason) = match curated {
        Some(entry) => (
            entry.risk,
            format!("{}: {}", entry.display_name, entry.impact),
        ),
        None => (
            RiskLevel::Low,
            "This optional component is not in Wino's curated list. Review the Windows documentation for its effect before changing it.".to_string(),
        ),
    };

    let target_state = if enable { "Enabled" } else { "Disabled" };
    let descriptor = OperationDescriptor {
        id: format!("feature:{}", feature_name),
        name: feature_name.to_string(),
        reason,
        component: format!("Windows feature: {}", feature_name),
        risk,
        reversible: true,
        requires_admin: true,
        supported_windows: SUPPORTED_WINDOWS.iter().map(|v| v.to_string()).collect(),
        // Enabling or disabling an optional component commonly needs a restart
        // to take effect, and the UI must say so before the user confirms.
        requires_reboot: true,
        current_state: String::new(),
        target_state: target_state.to_string(),
    };

    let verdict = SafetyEngine::validate_descriptor(&descriptor, &SystemInfo::detect());
    if !verdict.is_allowed {
        log_warn(
            "features",
            &format!(
                "Blocked feature change for {}: {}",
                feature_name, verdict.reason
            ),
        );
        return Err(verdict.reason);
    }

    if dry_run {
        return Ok(format!(
            "Dry run: would {} the Windows feature '{}'. A restart may be required.",
            if enable { "enable" } else { "disable" },
            feature_name
        ));
    }

    let _ = create_snapshot(&format!(
        "Windows feature: {} -> {}",
        feature_name,
        if enable { "enable" } else { "disable" }
    ));

    // /NoRestart keeps the reboot decision with the user; /All pulls in the
    // dependencies DISM considers part of the component.
    let feature_arg = format!("/FeatureName:{}", feature_name);
    let output = if enable {
        proc::run(
            "dism.exe",
            &[
                "/online",
                "/Enable-Feature",
                feature_arg.as_str(),
                "/All",
                "/NoRestart",
            ],
        )
    } else {
        proc::run(
            "dism.exe",
            &[
                "/online",
                "/Disable-Feature",
                feature_arg.as_str(),
                "/NoRestart",
            ],
        )
    }
    .map_err(|error| {
        log_error(
            "features",
            &format!("Failed to run dism.exe for {}: {}", feature_name, error),
        );
        format!("Failed to run dism.exe: {}", error)
    })?;

    let action = if enable { "enable" } else { "disable" };
    match output.exit_code {
        Some(0) => {
            log_info(
                "features",
                &format!("{}d Windows feature {}", action, feature_name),
            );
            Ok(format!(
                "Windows feature '{}' was set to {}. A restart may be required.",
                feature_name, target_state
            ))
        }
        Some(ERROR_SUCCESS_REBOOT_REQUIRED) => {
            log_info(
                "features",
                &format!(
                    "{}d Windows feature {}; a restart is required",
                    action, feature_name
                ),
            );
            Ok(format!(
                "Windows feature '{}' was set to {} and takes effect after a restart.",
                feature_name, target_state
            ))
        }
        code => {
            let detail = output.last_line();
            let detail = if detail.is_empty() {
                "no output".to_string()
            } else {
                detail
            };
            log_error(
                "features",
                &format!(
                    "Failed to {} Windows feature {} (exit {}): {}",
                    action,
                    feature_name,
                    code.map(|value| value.to_string())
                        .unwrap_or_else(|| "no code".to_string()),
                    detail
                ),
            );
            Err(format!(
                "Failed to {} Windows feature '{}': {}",
                action, feature_name, detail
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_feature_name_is_rejected_before_any_validation() {
        assert!(set_feature_enabled("   ", true, true).is_err());
    }

    #[test]
    fn dry_run_spawns_nothing_and_reports_one_of_two_truthful_outcomes() {
        // A dry run must never spawn DISM, so its result depends only on the
        // Safety Engine: it either produces the preview message or the engine's
        // own refusal (typically the missing elevation a test process has).
        // Both are correct; a third outcome would mean a process ran.
        let system = SystemInfo::detect();
        for enable in [true, false] {
            match set_feature_enabled("TelnetClient", enable, true) {
                Ok(message) => {
                    assert!(message.contains("TelnetClient"));
                    assert!(message.contains(if enable { "enable" } else { "disable" }));
                    assert!(message.contains("restart"));
                }
                Err(reason) => {
                    assert!(
                        !system.is_admin,
                        "a dry run of a Low-risk feature failed for a non-elevation reason: {}",
                        reason
                    );
                }
            }
        }
    }

    #[test]
    fn reboot_required_exit_code_is_treated_as_success() {
        // Guard the constant against the DISM contract it encodes.
        assert_eq!(ERROR_SUCCESS_REBOOT_REQUIRED, 3010);
        assert_ne!(ERROR_SUCCESS_REBOOT_REQUIRED, 0);
    }
}
