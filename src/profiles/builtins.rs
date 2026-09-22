//! Built-in profiles.
//!
//! Each built-in is a curated list of steps that reference operations Wino
//! already implements — debloat rule ids, privacy rule ids, service names, power
//! plans — so applying a built-in runs through the same Safety Engine, snapshot,
//! and audit path as applying that operation by hand.
//!
//! Every id, name, and risk level is read from `data/*.json` at call time rather
//! than duplicated here: a rule that is removed or reclassified in the data
//! files changes the profile with it, and no profile can reference a rule that
//! does not exist. Two things are genuinely local, because no data file declares
//! them: the Windows Game Mode registry pair (the values
//! [`crate::gaming::optimizer`] writes inline) and the visual-effects pair.
//!
//! No built-in touches Windows Defender, Windows Update, RPC, BFE, DHCP, or the
//! DNS Client. That is a design constraint, not a gap: those are the services
//! whose failure takes the machine down with it.

use crate::core::safety::RiskLevel;
use crate::debloat::rules::{load_default_rules, RegistryRuleItem};
use crate::privacy::policies::load_privacy_rules;
use crate::profiles::models::{Profile, ProfileOrigin, ProfileStep, StepKind};
use crate::services::scanner::load_service_rules;

/// Target id for the Windows Game Mode registry pair.
pub const GAME_MODE_TARGET: &str = "gaming_game_mode";
/// Target id for the visual-effects pair that favours responsiveness.
pub const VISUAL_FX_TARGET: &str = "visual_fx_performance";
/// Target id for the memory trim step.
pub const MEMORY_TRIM_TARGET: &str = "trim_working_sets";

/// Map a risk string from `data/service_rules.json` to a risk level.
fn risk_from_str(text: &str) -> RiskLevel {
    match text {
        "Low" => RiskLevel::Low,
        "Medium" => RiskLevel::Medium,
        "High" => RiskLevel::High,
        "Critical" => RiskLevel::Critical,
        _ => RiskLevel::Safe,
    }
}

/// One step per debloat rule in `preset`, using the rule's own id, name, and
/// declared risk.
fn debloat_steps_for_preset(preset: &str) -> Vec<ProfileStep> {
    load_default_rules()
        .into_iter()
        .filter(|rule| rule.preset == preset)
        .map(|rule| {
            ProfileStep::new(StepKind::Registry, &rule.id, &rule.name, rule.risk)
                .with_reason(&rule.reason)
        })
        .collect()
}

/// One step per privacy rule whose declared risk is `Safe`.
fn safe_privacy_steps() -> Vec<ProfileStep> {
    load_privacy_rules()
        .into_iter()
        .filter(|rule| rule.risk == RiskLevel::Safe)
        .map(|rule| {
            ProfileStep::new(StepKind::Privacy, &rule.id, &rule.name, rule.risk)
                .with_reason(&rule.reason)
        })
        .collect()
}

/// Service steps for every rule in `classification`, using the startup mode the
/// data file recommends.
fn service_steps_for_classification(classification: &str) -> Vec<ProfileStep> {
    load_service_rules()
        .into_iter()
        .filter(|rule| rule.classification == classification)
        .map(|rule| {
            let risk = risk_from_str(&rule.risk);
            let mut step = ProfileStep::new(
                StepKind::Service,
                &rule.service_name,
                &format!("{} -> {}", rule.display_name, rule.recommended_startup),
                risk,
            )
            .with_reason(&rule.reason)
            .with_value(&rule.recommended_startup);
            // The data file's recommendation is the action; nothing is toggled off.
            step.enable = true;
            step
        })
        .collect()
}

/// Registry values Wino writes but no data file declares.
///
/// Returns `None` for an unknown target so the caller can report a missing
/// operation instead of silently skipping it.
pub fn local_registry_values(target: &str) -> Option<Vec<RegistryRuleItem>> {
    fn dword(hive: &str, path: &str, name: &str, value: u32, restore: u32) -> RegistryRuleItem {
        RegistryRuleItem {
            hive: hive.to_string(),
            path: path.to_string(),
            value_name: name.to_string(),
            value_type: "DWORD".to_string(),
            value_data: value,
            restore_data: restore,
        }
    }

    const EXPLORER: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer";

    match target {
        // Same two values `crate::gaming::optimizer` writes inline.
        GAME_MODE_TARGET => Some(vec![
            dword(
                "HKCU",
                "Software\\Microsoft\\GameBar",
                "AutoGameModeEnabled",
                1,
                0,
            ),
            dword(
                "HKCU",
                "Software\\Microsoft\\GameBar",
                "AllowAutoGameMode",
                1,
                1,
            ),
        ]),
        VISUAL_FX_TARGET => Some(vec![
            // 2 = best performance, 0 = let Windows choose.
            dword(
                "HKCU",
                &format!("{}\\VisualEffects", EXPLORER),
                "VisualFXSetting",
                2,
                0,
            ),
            dword(
                "HKCU",
                &format!("{}\\Advanced", EXPLORER),
                "TaskbarAnimations",
                0,
                1,
            ),
            dword(
                "HKCU",
                &format!("{}\\Advanced", EXPLORER),
                "ListviewAlphaSelect",
                0,
                1,
            ),
            dword(
                "HKCU",
                &format!("{}\\Advanced", EXPLORER),
                "ListviewShadow",
                0,
                1,
            ),
            dword(
                "HKCU",
                "Software\\Microsoft\\Windows\\DWM",
                "EnableAeroPeek",
                0,
                1,
            ),
        ]),
        _ => None,
    }
}

/// The memory trim step shared by the gaming, performance, and low-RAM profiles.
fn memory_trim_step(reason: &str) -> ProfileStep {
    ProfileStep::new(
        StepKind::Memory,
        MEMORY_TRIM_TARGET,
        "Trim background working sets",
        RiskLevel::Safe,
    )
    .with_reason(reason)
}

/// Power plan step.
fn power_plan_step(alias: &str, label: &str, reason: &str) -> ProfileStep {
    ProfileStep::new(StepKind::Power, alias, label, RiskLevel::Low).with_reason(reason)
}

/// Power setting step: `target` is an id from
/// [`crate::power::models::EXPOSED_SETTING_IDS`], `value` the requested number.
fn power_setting_step(id: &str, label: &str, value: u32, reason: &str) -> ProfileStep {
    ProfileStep::new(StepKind::Power, id, label, RiskLevel::Low)
        .with_value(&value.to_string())
        .with_reason(reason)
}

fn balanced() -> Profile {
    let mut profile = Profile::new(
        "balanced",
        "Balanced",
        "Every Safe-preset debloat rule, the Balanced power plan, and a memory trim. \
         Keeps Windows Update, Defender, RPC, BFE, DHCP, and the DNS Client untouched; \
         no package removal beyond the Safe preset's promotional app stubs; no service \
         is set to Disabled.",
        ProfileOrigin::BuiltIn,
    );
    profile.is_default = true;
    for step in debloat_steps_for_preset("Safe") {
        profile = profile.with_step(step);
    }
    profile
        .with_step(power_plan_step(
            "balanced",
            "Balanced power plan",
            "Balanced is the profile Windows itself tunes for a mix of responsiveness and power draw.",
        ))
        .with_step(memory_trim_step(
            "Returns idle working sets to the standby list; nothing is written to disk.",
        ))
}

fn gaming() -> Profile {
    let mut profile = Profile::new(
        "gaming",
        "Gaming",
        "High Performance power plan, Windows Game Mode, a memory trim, and the Xbox Live \
         services the data file marks Optional set to Manual so they start on demand. \
         Does not touch Windows Update or Defender, does not uninstall Xbox components, \
         and does not disable any service permanently.",
        ProfileOrigin::BuiltIn,
    );

    profile = profile.with_step(power_plan_step(
        "high_performance",
        "High Performance power plan",
        "Keeps the processor above its idle states while gaming.",
    ));
    profile = profile.with_step(
        ProfileStep::new(
            StepKind::Registry,
            GAME_MODE_TARGET,
            "Enable Windows Game Mode",
            RiskLevel::Low,
        )
        .with_reason(
            "Lets Windows prioritise the foreground game and suppress background notifications.",
        ),
    );
    profile = profile.with_step(memory_trim_step(
        "Frees background working sets so the game has more physical memory to work with.",
    ));

    for step in service_steps_for_classification("Optional") {
        // Only the Xbox Live family: the other Optional rule (SysMain) is
        // recommended Automatic and helps, not hurts, game load times.
        let is_xbox = step.target.starts_with("Xbl") || step.target.starts_with("Xbox");
        if is_xbox {
            profile = profile.with_step(step);
        }
    }

    profile
}

fn performance() -> Profile {
    let mut profile = Profile::new(
        "performance",
        "Performance",
        "High Performance power plan, visual effects reduced to best performance, the \
         processor ceiling unlocked to 100 % on AC and DC, and a memory trim. \
         Does not overclock, does not change the processor floor, does not disable \
         services, and does not remove any app.",
        ProfileOrigin::BuiltIn,
    );

    profile = profile.with_step(power_plan_step(
        "high_performance",
        "High Performance power plan",
        "Removes the power plan's aggressive idle throttling.",
    ));
    profile = profile.with_step(
        ProfileStep::new(
            StepKind::Visual,
            VISUAL_FX_TARGET,
            "Reduce visual effects to best performance",
            RiskLevel::Low,
        )
        .with_reason(
            "Disables taskbar, list-view, and Aero Peek animations that cost GPU and CPU time.",
        ),
    );
    profile = profile.with_step(power_setting_step(
        "processor_max",
        "Processor maximum state 100 %",
        100,
        "Allows the processor to reach its full rated frequency on battery as well as on AC.",
    ));
    profile = profile.with_step(memory_trim_step(
        "Frees background working sets for the foreground workload.",
    ));

    profile
}

fn battery_saver() -> Profile {
    let mut profile = Profile::new(
        "battery_saver",
        "Battery Saver",
        "Power Saver plan, shorter display and sleep timeouts, the processor ceiling \
         limited to 50 %, and USB selective suspend enabled. \
         Does not disable the display entirely, does not hibernate, does not change \
         the processor floor, and does not disable Bluetooth or Wi-Fi adapters.",
        ProfileOrigin::BuiltIn,
    );

    profile = profile.with_step(power_plan_step(
        "power_saver",
        "Power Saver power plan",
        "Windows' own low-power plan, with lower clocks and more aggressive idle.",
    ));
    profile = profile.with_step(power_setting_step(
        "display_timeout",
        "Display off after 5 minutes",
        300,
        "The display is the single largest battery drain on a laptop.",
    ));
    profile = profile.with_step(power_setting_step(
        "sleep_timeout",
        "Sleep after 15 minutes",
        900,
        "Suspends the session instead of idling at full power draw.",
    ));
    profile = profile.with_step(power_setting_step(
        "processor_max",
        "Processor maximum state 50 %",
        50,
        "Halves the processor's power ceiling, the largest single saving on battery.",
    ));
    profile = profile.with_step(power_setting_step(
        "usb_suspend",
        "USB selective suspend enabled",
        1,
        "Powers down idle USB devices instead of keeping their buses awake.",
    ));

    profile
}

fn privacy() -> Profile {
    let mut profile = Profile::new(
        "privacy",
        "Privacy",
        "Every privacy rule the data file rates Safe: advertising ID, activity history, \
         tailored experiences, inking and typing personalization, and feedback prompts. \
         Does not disable telemetry services, does not block Windows Update, does not \
         remove apps, and does not change any power setting.",
        ProfileOrigin::BuiltIn,
    );

    for step in safe_privacy_steps() {
        profile = profile.with_step(step);
    }

    profile
}

fn low_ram() -> Profile {
    let mut profile = Profile::new(
        "low_ram",
        "Low RAM",
        "A memory trim plus every service the data file marks Safe to change, set to its \
         recommended startup mode. Aimed at 4 GB machines. \
         Does not enable a pagefile change, does not disable SysMain (the data file rates \
         it Medium and recommends Automatic), does not remove apps, and does not touch \
         Defender or Windows Update.",
        ProfileOrigin::BuiltIn,
    );

    profile = profile.with_step(memory_trim_step(
        "Releases idle working sets, the largest immediate win on a small-RAM machine.",
    ));
    for step in service_steps_for_classification("Safe to change") {
        profile = profile.with_step(step);
    }

    profile
}

/// The six shipped profiles, in display order.
pub fn builtin_profiles() -> Vec<Profile> {
    vec![
        balanced(),
        gaming(),
        performance(),
        battery_saver(),
        privacy(),
        low_ram(),
    ]
}

/// The profile applied when the user names none.
pub fn default_profile() -> Profile {
    balanced()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_builtin_id_has_a_profile_with_steps() {
        let profiles = builtin_profiles();
        assert_eq!(profiles.len(), crate::profiles::models::BUILTIN_IDS.len());

        for (profile, expected_id) in profiles.iter().zip(crate::profiles::models::BUILTIN_IDS) {
            assert_eq!(&profile.id, expected_id);
            assert_eq!(profile.origin, ProfileOrigin::BuiltIn);
            assert!(!profile.name.is_empty());
            assert!(
                profile.steps.len() >= 3,
                "{} must have real steps, has {}",
                profile.id,
                profile.steps.len()
            );
            for step in &profile.steps {
                assert!(
                    !step.target.trim().is_empty(),
                    "{} has a step with an empty target",
                    profile.id
                );
                assert!(
                    !step.label.trim().is_empty(),
                    "{} has an unlabelled step",
                    profile.id
                );
            }
            assert!(profile.validate().is_ok(), "{} must validate", profile.id);
        }
    }

    #[test]
    fn exactly_one_builtin_is_the_default_and_it_is_balanced() {
        let defaults: Vec<String> = builtin_profiles()
            .into_iter()
            .filter(|p| p.is_default)
            .map(|p| p.id)
            .collect();
        assert_eq!(defaults, vec!["balanced".to_string()]);
        assert_eq!(default_profile().id, "balanced");
    }

    #[test]
    fn no_builtin_targets_a_critical_service_or_blocked_step() {
        let rules = load_service_rules();
        let critical: Vec<&str> = rules
            .iter()
            .filter(|r| r.classification == "Do not touch")
            .map(|r| r.service_name.as_str())
            .collect();
        assert!(
            critical.contains(&"WinDefend"),
            "data file must still guard Defender"
        );

        for profile in builtin_profiles() {
            assert!(
                profile.blocked_steps().is_empty(),
                "{} contains a Critical step",
                profile.id
            );
            for step in &profile.steps {
                if step.kind == StepKind::Service {
                    assert!(
                        !critical
                            .iter()
                            .any(|c| c.eq_ignore_ascii_case(&step.target)),
                        "{} tries to change protected service {}",
                        profile.id,
                        step.target
                    );
                }
            }
        }
    }

    #[test]
    fn debloat_and_privacy_steps_reference_existing_rule_ids() {
        let debloat_ids: Vec<String> = load_default_rules().into_iter().map(|r| r.id).collect();
        let privacy_ids: Vec<String> = load_privacy_rules().into_iter().map(|r| r.id).collect();

        for profile in builtin_profiles() {
            for step in &profile.steps {
                match step.kind {
                    StepKind::Registry => assert!(
                        debloat_ids.contains(&step.target)
                            || local_registry_values(&step.target).is_some(),
                        "{} references unknown registry target {}",
                        profile.id,
                        step.target
                    ),
                    StepKind::Privacy => assert!(
                        privacy_ids.contains(&step.target),
                        "{} references unknown privacy rule {}",
                        profile.id,
                        step.target
                    ),
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn service_steps_carry_a_startup_mode_the_manager_can_parse() {
        for profile in builtin_profiles() {
            for step in &profile.steps {
                if step.kind == StepKind::Service {
                    assert!(
                        matches!(step.value.as_str(), "Manual" | "Disabled" | "Automatic"),
                        "{} has unparseable startup mode '{}'",
                        profile.id,
                        step.value
                    );
                }
            }
        }
    }

    #[test]
    fn power_steps_use_known_plan_aliases_or_exposed_setting_ids() {
        for profile in builtin_profiles() {
            for step in &profile.steps {
                if step.kind != StepKind::Power {
                    continue;
                }
                let is_plan = crate::power::manager::resolve_plan_alias(&step.target).is_some();
                let is_setting =
                    crate::power::models::EXPOSED_SETTING_IDS.contains(&step.target.as_str());
                assert!(
                    is_plan || is_setting,
                    "{} has power step '{}' that is neither a plan nor an exposed setting",
                    profile.id,
                    step.target
                );
                if is_setting {
                    assert!(
                        step.value.parse::<u32>().is_ok(),
                        "{} setting step {} needs a numeric value",
                        profile.id,
                        step.target
                    );
                }
            }
        }
    }

    #[test]
    fn balanced_power_step_names_a_real_scheme_constant() {
        let balanced = builtin_profiles()
            .into_iter()
            .find(|p| p.id == "balanced")
            .expect("balanced exists");
        let power = balanced
            .steps
            .iter()
            .find(|s| s.kind == StepKind::Power)
            .expect("balanced has a power step");
        assert_eq!(
            crate::power::manager::resolve_plan_alias(&power.target),
            Some(crate::power::models::scheme_guids::BALANCED)
        );
    }

    #[test]
    fn local_registry_values_are_unknown_targets_only() {
        assert!(local_registry_values("no_such_target").is_none());

        let game_mode = local_registry_values(GAME_MODE_TARGET).expect("game mode values");
        assert_eq!(game_mode.len(), 2);
        assert!(game_mode
            .iter()
            .all(|v| v.hive == "HKCU" && v.value_data == 1));

        let visual = local_registry_values(VISUAL_FX_TARGET).expect("visual values");
        assert!(visual.len() >= 4);
        for item in &visual {
            assert!(!item.path.trim().is_empty());
            assert!(!item.value_name.trim().is_empty());
            assert_eq!(item.value_type, "DWORD");
            assert_ne!(
                item.value_data, item.restore_data,
                "restore value must undo the write"
            );
        }
    }
}
