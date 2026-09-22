//! Profile Engine: load, apply, and persist profiles.
//!
//! A profile is not a new kind of operation — it is a list of existing ones.
//! Applying a profile therefore delegates to the same functions the individual
//! views call, which means every step inherits the Safety Engine gate, the
//! pre-change snapshot, and the audit log for free.
//!
//! Two invariants shape this module:
//!
//! * a step the Safety Engine blocks is *skipped*, never attempted, and is
//!   reported through `StepOutcome::skipped_reason`;
//! * a step that was attempted and failed is *failed*, with `skipped_reason`
//!   left `None`, so [`ProfileApplyReport`] can count the two apart.

use crate::core::config::AppConfig;
use crate::core::executor::SystemExecutor;
use crate::core::logger::{log_error, log_info, log_warn};
use crate::core::safety::{OperationDescriptor, RiskLevel, SafetyEngine};
use crate::core::system::SystemInfo;
use crate::debloat::rules::{load_default_rules, DebloatRule};
use crate::privacy::policies::{load_privacy_rules, PrivacyRule};
use crate::profiles::builtins::{builtin_profiles, local_registry_values};
use crate::profiles::models::{
    Profile, ProfileApplyReport, ProfileOrigin, ProfileStep, StepKind, StepOutcome,
};
use crate::restore::snapshots::{create_snapshot, create_snapshot_for_profile, ProfileBackupEntry};
use crate::services::manager::StartupType;
use std::fs;
use std::path::PathBuf;

/// Where user profiles live: `%APPDATA%\Wino\profiles`.
pub fn profiles_dir() -> PathBuf {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        let path = PathBuf::from(appdata).join("Wino").join("profiles");
        let _ = fs::create_dir_all(&path);
        path
    } else {
        let path = PathBuf::from("profiles");
        let _ = fs::create_dir_all(&path);
        path
    }
}

/// Filename for a profile id.
///
/// The id inside the TOML stays authoritative; the filename is slugified so an
/// imported profile with an exotic id cannot escape the profiles directory.
fn profile_file(id: &str) -> PathBuf {
    profiles_dir().join(format!("{}.toml", crate::profiles::models::slugify(id)))
}

/// Built-ins first, then user profiles from disk.
///
/// A user profile whose id collides with a built-in is **skipped** with a
/// warning rather than merged: silently shadowing a built-in would make the
/// shipped definition unreachable and the collision invisible.
pub fn load_profiles() -> Vec<Profile> {
    let mut profiles = builtin_profiles();
    let builtin_ids: Vec<String> = profiles.iter().map(|p| p.id.clone()).collect();

    let Ok(entries) = fs::read_dir(profiles_dir()) else {
        return profiles;
    };

    let mut user_profiles: Vec<Profile> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.extension().map(|e| e == "toml").unwrap_or(false) {
            continue;
        }
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        match toml::from_str::<Profile>(&content) {
            Ok(profile) if builtin_ids.contains(&profile.id) => {
                log_warn(
                    "profiles",
                    &format!(
                        "User profile '{}' in {} collides with a built-in id and was skipped. Rename it to load it.",
                        profile.id,
                        path.display()
                    ),
                );
            }
            Ok(mut profile) => {
                profile.origin = ProfileOrigin::User;
                user_profiles.push(profile);
            }
            Err(err) => log_warn(
                "profiles",
                &format!("Could not parse profile {}: {}", path.display(), err),
            ),
        }
    }

    user_profiles.sort_by_key(|a| a.name.to_lowercase());
    profiles.extend(user_profiles);
    profiles
}

/// Rule data a step resolves against, loaded once per apply or preview.
struct RuleIndex {
    debloat: Vec<DebloatRule>,
    privacy: Vec<PrivacyRule>,
}

impl RuleIndex {
    fn load() -> Self {
        Self {
            debloat: load_default_rules(),
            privacy: load_privacy_rules(),
        }
    }

    fn debloat_rule(&self, id: &str) -> Option<&DebloatRule> {
        self.debloat.iter().find(|rule| rule.id == id)
    }

    fn privacy_rule(&self, id: &str) -> Option<&PrivacyRule> {
        self.privacy.iter().find(|rule| rule.id == id)
    }

    /// Descriptor for a step backed by a rule, using the rule's own metadata.
    ///
    /// The effective risk is the higher of the rule's and the step's, so a
    /// profile cannot under-declare a rule's risk and slip past the gate.
    fn debloat_descriptor(&self, step: &ProfileStep) -> Option<OperationDescriptor> {
        let rule = self.debloat_rule(&step.target)?;
        Some(OperationDescriptor {
            id: rule.id.clone(),
            name: rule.name.clone(),
            reason: rule.reason.clone(),
            component: format!("Registry ({})", rule.category),
            risk: rule.risk.max(step.risk),
            reversible: rule.reversible,
            requires_admin: rule.requires_admin,
            supported_windows: rule.supported_windows.clone(),
            requires_reboot: false,
            current_state: String::new(),
            target_state: String::new(),
        })
    }

    fn privacy_descriptor(&self, step: &ProfileStep) -> Option<OperationDescriptor> {
        let rule = self.privacy_rule(&step.target)?;
        Some(OperationDescriptor {
            id: rule.id.clone(),
            name: rule.name.clone(),
            reason: rule.reason.clone(),
            component: format!("Privacy ({})", rule.category),
            risk: rule.risk.max(step.risk),
            reversible: rule.reversible,
            requires_admin: rule.requires_admin,
            supported_windows: rule.supported_windows.clone(),
            requires_reboot: false,
            current_state: String::new(),
            target_state: String::new(),
        })
    }
}

/// True when the step targets a power setting rather than a power plan.
fn is_power_setting_step(step: &ProfileStep) -> bool {
    crate::power::models::EXPOSED_SETTING_IDS.contains(&step.target.as_str())
}

/// Descriptor for a step with no declarative rule behind it.
fn step_descriptor(
    step: &ProfileStep,
    component: &str,
    requires_admin: bool,
) -> OperationDescriptor {
    OperationDescriptor::simple(&step.target, &step.label, component, step.risk)
        .with_reason(&step.reason)
        .with_admin(requires_admin)
        .with_windows(&["10", "11"])
        .with_reversible(true)
}

/// Describe one step for the confirmation dialog.
fn descriptor_for_step(step: &ProfileStep, rules: &RuleIndex) -> OperationDescriptor {
    match step.kind {
        StepKind::Registry => rules
            .debloat_descriptor(step)
            .unwrap_or_else(|| step_descriptor(step, &format!("Registry: {}", step.target), true)),
        StepKind::Privacy => rules
            .privacy_descriptor(step)
            .unwrap_or_else(|| step_descriptor(step, &format!("Privacy: {}", step.target), true)),
        StepKind::Power => {
            let component = if is_power_setting_step(step) {
                format!("Power setting: {}", step.target)
            } else {
                format!("Power plan: {}", step.target)
            };
            step_descriptor(step, &component, true)
        }
        StepKind::Service => step_descriptor(step, &format!("Service: {}", step.target), true),
        StepKind::Task => step_descriptor(step, &format!("Scheduled task: {}", step.target), true),
        StepKind::Startup => {
            step_descriptor(step, &format!("Startup entry: {}", step.target), false)
        }
        StepKind::Memory => step_descriptor(step, "Memory: working set trim", false),
        StepKind::Visual => step_descriptor(step, "Visual effects", false),
    }
}

/// One descriptor per step, in profile order. This is what the confirmation
/// dialog renders.
pub fn preview_profile(profile: &Profile) -> Vec<OperationDescriptor> {
    let rules = RuleIndex::load();
    profile
        .steps
        .iter()
        .map(|step| descriptor_for_step(step, &rules))
        .collect()
}

/// Map a profile step's startup mode onto the service manager's enum.
fn startup_type_for(step: &ProfileStep) -> StartupType {
    match step.value.to_lowercase().as_str() {
        "disabled" => StartupType::Disabled,
        "automatic" => StartupType::Automatic,
        "manual" => StartupType::Manual,
        _ => {
            if step.enable {
                StartupType::Manual
            } else {
                StartupType::Disabled
            }
        }
    }
}

/// Write the registry values Wino defines locally rather than in a data file.
///
/// Used for the Game Mode and visual-effects targets, which have no debloat rule
/// behind them.
fn write_local_registry_values(
    target: &str,
    description: &str,
    dry_run: bool,
) -> Result<(), String> {
    let Some(values) = local_registry_values(target) else {
        return Err(format!("No registry definition for '{}'.", target));
    };

    if !dry_run {
        let _ = create_snapshot(description);
    }

    let mut failures = Vec::new();
    for value in &values {
        let result = SystemExecutor::set_registry_dword(
            &value.hive,
            &value.path,
            &value.value_name,
            value.value_data,
            dry_run,
        );
        if !result.success {
            failures.push(result.details);
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}

/// Run one step, returning `Err(reason)` for a genuine failure.
fn execute_step(step: &ProfileStep, rules: &RuleIndex, dry_run: bool) -> Result<(), String> {
    match step.kind {
        StepKind::Registry => match rules.debloat_rule(&step.target) {
            Some(rule) => {
                first_failure(crate::debloat::executor::apply_debloat_rule(rule, dry_run))
            }
            None => write_local_registry_values(
                &step.target,
                &format!("Registry: {}", step.target),
                dry_run,
            ),
        },
        StepKind::Privacy => {
            let Some(rule) = rules.privacy_rule(&step.target) else {
                return Err(format!("Privacy rule '{}' not found.", step.target));
            };
            first_failure(crate::privacy::scanner::apply_privacy_rule(
                rule,
                step.enable,
                dry_run,
            ))
        }
        StepKind::Service => {
            crate::services::manager::set_service_startup(&step.target, startup_type_for(step))
        }
        StepKind::Power => {
            if is_power_setting_step(step) {
                let value: u32 = step.value.trim().parse().map_err(|_| {
                    format!("Power setting step '{}' has no numeric value.", step.target)
                })?;
                let settings = crate::power::settings::read_all_settings();
                let Some(setting) = settings.iter().find(|s| s.id == step.target) else {
                    return Err(format!(
                        "Power setting '{}' is not exposed by the active scheme.",
                        step.target
                    ));
                };
                return crate::power::settings::write_setting(setting, value, value, dry_run)
                    .map(|_| ());
            }
            crate::power::manager::set_active_plan(&step.target, dry_run).map(|_| ())
        }
        StepKind::Task => {
            crate::tasks::manager::set_task_enabled(&step.target, step.enable, dry_run)
        }
        StepKind::Memory => {
            let report = crate::memory::optimizer::optimize_memory(dry_run);
            log_info("profiles", &report.message);
            Ok(())
        }
        StepKind::Visual => write_local_registry_values(
            &step.target,
            &format!("Visual effects: {}", step.label),
            dry_run,
        ),
        StepKind::Startup => {
            let Some(item) = crate::startup::scanner::scan_startup_items()
                .into_iter()
                .find(|item| item.name.eq_ignore_ascii_case(&step.target))
            else {
                return Err(format!("Startup entry '{}' was not found.", step.target));
            };
            crate::startup::manager::toggle_startup_item(&item, step.enable, dry_run)
        }
    }
}

/// Collapse a batch of execution results into the first failure, if any.
fn first_failure(results: Vec<crate::core::executor::ExecutionResult>) -> Result<(), String> {
    if results.is_empty() {
        return Ok(());
    }
    let failures: Vec<String> = results
        .into_iter()
        .filter(|result| !result.success)
        .map(|result| result.details)
        .collect();
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}

/// Apply every step of `profile` that the Safety Engine allows.
///
/// The report never claims complete success when a step failed or was skipped:
/// [`ProfileApplyReport::summary`] renders the counts, and the caller shows that
/// string verbatim.
pub fn apply_profile(profile: &Profile, dry_run: bool) -> ProfileApplyReport {
    let mut report = ProfileApplyReport {
        profile_id: profile.id.clone(),
        profile_name: profile.name.clone(),
        outcomes: Vec::new(),
        message: String::new(),
        snapshot_id: None,
    };

    if let Err(err) = profile.validate() {
        report.message = format!("Profile '{}' is not valid: {}", profile.id, err);
        log_error("profiles", &report.message);
        return report;
    }

    let rules = RuleIndex::load();
    let sys_info = SystemInfo::detect();

    // 1. Gate every step through the Safety Engine before anything runs.
    let mut allowed: Vec<&ProfileStep> = Vec::new();
    for step in &profile.steps {
        let descriptor = descriptor_for_step(step, &rules);
        let check = SafetyEngine::validate_descriptor(&descriptor, &sys_info);
        if check.is_allowed {
            allowed.push(step);
        } else {
            log_warn(
                "profiles",
                &format!("Skipping step '{}': {}", step.label, check.reason),
            );
            report.outcomes.push(StepOutcome {
                label: step.label.clone(),
                kind: step.kind,
                success: false,
                skipped_reason: Some(check.reason),
            });
        }
    }

    // 2. Snapshot before the first mutation. A profile with nothing to do gets
    //    no snapshot — an empty restore point is noise in the restore list.
    if !dry_run && !allowed.is_empty() {
        let applied_steps: Vec<String> = allowed.iter().map(|s| s.target.clone()).collect();
        let snapshot = create_snapshot_for_profile(
            &format!("Profile: {}", profile.name),
            &ProfileBackupEntry {
                profile_id: profile.id.clone(),
                profile_name: profile.name.clone(),
                applied_steps,
            },
        );
        report.snapshot_id = Some(snapshot.id.clone());

        // System Protection is often off; that is not a reason to refuse the
        // whole profile, so the failure is logged and the apply continues.
        if AppConfig::load().general.auto_create_restore_point
            && profile.max_risk() >= RiskLevel::Low
        {
            match crate::restore::vss_point::create_windows_restore_point(&format!(
                "Wino profile: {}",
                profile.name
            )) {
                Ok(message) => log_info("profiles", &message),
                Err(err) => log_warn(
                    "profiles",
                    &format!("VSS restore point unavailable, continuing: {}", err),
                ),
            }
        }
    }

    // 3. Execute. A failed step keeps `skipped_reason: None` — that field is
    //    reserved for steps the Safety Engine never let run.
    for step in allowed {
        let result = execute_step(step, &rules, dry_run);
        if let Err(err) = &result {
            log_error(
                "profiles",
                &format!("Step '{}' failed: {}", step.label, err),
            );
        }
        report.outcomes.push(StepOutcome {
            label: step.label.clone(),
            kind: step.kind,
            success: result.is_ok(),
            skipped_reason: None,
        });
    }

    report.message = report.summary();
    log_info("profiles", &report.message);
    report
}

/// Refuse an id owned by a built-in profile.
fn reject_builtin_id(id: &str, action: &str) -> Result<(), String> {
    if builtin_profiles().iter().any(|p| p.id == id) {
        return Err(format!(
            "'{}' is a built-in profile id and cannot be {}. Duplicate it under a new name to customise it.",
            id, action
        ));
    }
    Ok(())
}

/// Persist a user profile as TOML. Built-in ids are refused.
pub fn save_user_profile(profile: &Profile) -> Result<(), String> {
    profile.validate()?;
    reject_builtin_id(&profile.id, "overwritten")?;

    let mut stored = profile.clone();
    stored.origin = ProfileOrigin::User;

    let content = toml::to_string_pretty(&stored)
        .map_err(|e| format!("Failed to serialize profile '{}': {}", stored.id, e))?;
    let path = profile_file(&stored.id);
    fs::write(&path, content)
        .map_err(|e| format!("Failed to write '{}': {}", path.display(), e))?;

    log_info(
        "profiles",
        &format!("Saved profile '{}' to {}", stored.id, path.display()),
    );
    Ok(())
}

/// Delete a user profile. Built-in profiles cannot be deleted.
pub fn delete_user_profile(id: &str) -> Result<(), String> {
    reject_builtin_id(id, "deleted")?;

    let path = profile_file(id);
    if !path.exists() {
        return Err(format!("No user profile with id '{}' exists.", id));
    }

    fs::remove_file(&path).map_err(|e| format!("Failed to delete '{}': {}", path.display(), e))?;
    log_info("profiles", &format!("Deleted profile '{}'", id));
    Ok(())
}

/// Export a profile as TOML to `destination`.
///
/// A directory destination receives `<id>.toml`; any other path is used as
/// given. Returns the path actually written.
pub fn export_profile(profile: &Profile, destination: &str) -> Result<String, String> {
    profile.validate()?;

    let content = toml::to_string_pretty(profile)
        .map_err(|e| format!("Failed to serialize profile '{}': {}", profile.id, e))?;

    let mut path = PathBuf::from(destination);
    if path.is_dir() {
        path = path.join(format!(
            "{}.toml",
            crate::profiles::models::slugify(&profile.id)
        ));
    }

    fs::write(&path, content)
        .map_err(|e| format!("Failed to write '{}': {}", path.display(), e))?;

    let message = format!("Profile '{}' exported to {}.", profile.name, path.display());
    log_info("profiles", &message);
    Ok(message)
}

/// Read a profile from a TOML file.
///
/// The imported profile is always a user profile, and an id that already exists
/// is refused rather than overwriting the existing definition.
pub fn import_profile(path: &str) -> Result<Profile, String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read '{}': {}", path, e))?;
    let mut profile: Profile = toml::from_str(&content)
        .map_err(|e| format!("'{}' is not a valid profile: {}", path, e))?;

    profile.validate()?;
    profile.origin = ProfileOrigin::User;

    if let Some(existing) = load_profiles().into_iter().find(|p| p.id == profile.id) {
        return Err(format!(
            "A profile with id '{}' already exists ('{}'). Rename the imported profile before importing it.",
            profile.id, existing.name
        ));
    }

    log_info(
        "profiles",
        &format!("Imported profile '{}' from {}", profile.id, path),
    );
    Ok(profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(kind: StepKind, target: &str, risk: RiskLevel) -> ProfileStep {
        ProfileStep::new(kind, target, target, risk)
    }

    #[test]
    fn preview_produces_one_descriptor_per_step_in_order() {
        let profile = Profile::new("p", "P", "", ProfileOrigin::User)
            .with_step(step(
                StepKind::Registry,
                "disable_bing_search_start",
                RiskLevel::Safe,
            ))
            .with_step(step(StepKind::Power, "high_performance", RiskLevel::Low))
            .with_step(step(StepKind::Memory, "trim_working_sets", RiskLevel::Safe));

        let preview = preview_profile(&profile);
        assert_eq!(preview.len(), profile.steps.len());
        assert_eq!(preview[0].id, "disable_bing_search_start");
        assert_eq!(preview[0].risk, RiskLevel::Safe);
        assert!(!preview[0].supported_windows.is_empty());
        assert!(preview[1].requires_admin);
        assert_eq!(preview[2].component, "Memory: working set trim");
    }

    #[test]
    fn descriptor_risk_is_never_lower_than_the_backing_rule() {
        // "disable_widgets_feed" is declared Low in data/debloat_rules.json.
        let understated =
            Profile::new("p", "P", "", ProfileOrigin::User).with_step(ProfileStep::new(
                StepKind::Registry,
                "disable_widgets_feed",
                "Widgets",
                RiskLevel::Safe,
            ));
        let descriptor = preview_profile(&understated)
            .into_iter()
            .next()
            .expect("one descriptor");
        assert_eq!(descriptor.risk, RiskLevel::Low);
    }

    #[test]
    fn critical_step_is_skipped_and_never_attempted() {
        let profile = Profile::new("p", "P", "", ProfileOrigin::User)
            .with_step(step(
                StepKind::Registry,
                "not_a_rule_at_all",
                RiskLevel::Critical,
            ))
            .with_step(step(StepKind::Memory, "trim_working_sets", RiskLevel::Safe));

        let report = apply_profile(&profile, true);
        assert_eq!(report.outcomes.len(), 2);
        assert_eq!(report.skipped_count(), 1);
        assert_eq!(report.failed_count(), 0);
        assert_eq!(report.applied_count(), 1);

        let skipped = report
            .outcomes
            .iter()
            .find(|o| o.skipped_reason.is_some())
            .expect("skipped");
        assert!(!skipped.success);
        assert!(
            skipped
                .skipped_reason
                .as_deref()
                .unwrap_or("")
                .contains("core stability"),
            "skip reason must come from the Safety Engine: {:?}",
            skipped.skipped_reason
        );
        assert!(report.is_partial());
        assert!(report.summary().contains("1 skipped"));
    }

    #[test]
    fn invalid_profile_is_rejected_without_running_anything() {
        let mut profile = Profile::new("p", "P", "", ProfileOrigin::User).with_step(step(
            StepKind::Memory,
            "trim_working_sets",
            RiskLevel::Safe,
        ));
        profile.name = "   ".to_string();

        let report = apply_profile(&profile, true);
        assert!(report.outcomes.is_empty());
        assert!(report.message.contains("not valid"));
    }

    #[test]
    fn builtin_ids_are_locked_against_save_and_delete() {
        assert!(save_user_profile(&crate::profiles::builtins::default_profile()).is_err());
        let err = delete_user_profile("balanced").unwrap_err();
        assert!(err.contains("built-in"), "{}", err);
    }

    #[test]
    fn profile_round_trips_through_toml() {
        let profile = Profile::new(
            "my_tweaks",
            "My Tweaks",
            "Hand-picked.",
            ProfileOrigin::User,
        )
        .with_step(step(
            StepKind::Registry,
            "disable_bing_search_start",
            RiskLevel::Safe,
        ))
        .with_step(
            ProfileStep::new(
                StepKind::Power,
                "processor_max",
                "Processor max",
                RiskLevel::Low,
            )
            .with_value("100"),
        );

        let text = toml::to_string_pretty(&profile).expect("serialize");
        let back: Profile = toml::from_str(&text).expect("deserialize");
        assert_eq!(back.id, profile.id);
        assert_eq!(back.origin, ProfileOrigin::User);
        assert_eq!(back.steps.len(), 2);
        assert_eq!(back.steps[1].value, "100");
        assert!(back.validate().is_ok());
    }

    #[test]
    fn startup_mode_maps_onto_the_service_enum() {
        let mut manual = step(StepKind::Service, "MapsBroker", RiskLevel::Safe);
        manual.value = "Manual".to_string();
        assert_eq!(startup_type_for(&manual), StartupType::Manual);

        let mut disabled = step(StepKind::Service, "RetailDemo", RiskLevel::Safe);
        disabled.value = "Disabled".to_string();
        assert_eq!(startup_type_for(&disabled), StartupType::Disabled);

        // No declared mode falls back to the step's toggle.
        let toggled = step(StepKind::Service, "XblAuthManager", RiskLevel::Low).disabling();
        assert_eq!(startup_type_for(&toggled), StartupType::Disabled);
    }

    #[test]
    fn profiles_dir_is_under_the_wino_config_directory() {
        let dir = profiles_dir();
        assert!(dir.ends_with("profiles"));
        assert!(dir.exists());
    }
}
