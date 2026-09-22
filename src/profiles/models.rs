//! Profile Engine data model.
//!
//! A profile is a named, ordered list of steps. Each step names an *existing*
//! Wino operation — a debloat rule id, a service change, a power plan — rather
//! than re-implementing it. Applying a profile therefore routes through the
//! same Safety Engine, snapshots, and audit logging as applying that operation
//! by hand.

use crate::core::safety::RiskLevel;
use serde::{Deserialize, Serialize};

/// What a profile step operates on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StepKind {
    /// Debloat rule id from `data/debloat_rules.json`.
    Registry,
    /// Service startup change.
    Service,
    /// Power plan selection.
    Power,
    /// Scheduled task enable/disable.
    Task,
    /// Memory optimization behavior.
    Memory,
    /// Visual effects for performance.
    Visual,
    /// Privacy rule id from `data/privacy_rules.json`.
    Privacy,
    /// Startup entry enable/disable.
    Startup,
}

impl StepKind {
    pub fn i18n_key(&self) -> &'static str {
        match self {
            StepKind::Registry => "profiles.step_registry",
            StepKind::Service => "profiles.step_service",
            StepKind::Power => "profiles.step_power",
            StepKind::Task => "profiles.step_task",
            StepKind::Memory => "profiles.step_memory",
            StepKind::Visual => "profiles.step_visual",
            StepKind::Privacy => "profiles.step_privacy",
            StepKind::Startup => "profiles.step_startup",
        }
    }

    pub fn color_rgb(&self) -> (u8, u8, u8) {
        match self {
            StepKind::Registry => (60, 144, 255),
            StepKind::Service => (78, 222, 163),
            StepKind::Power => (250, 204, 21),
            StepKind::Task => (255, 185, 95),
            StepKind::Memory => (167, 139, 250),
            StepKind::Visual => (56, 189, 248),
            StepKind::Privacy => (236, 72, 153),
            StepKind::Startup => (16, 185, 129),
        }
    }
}

/// One operation inside a profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileStep {
    pub kind: StepKind,
    /// Target identifier interpreted per `kind`:
    /// debloat rule id, service name, power plan guid or alias, task path,
    /// privacy rule id, startup entry name.
    pub target: String,
    /// Human label shown in the preview list.
    pub label: String,
    /// Enable (`true`) or disable (`false`) for toggle-style steps.
    #[serde(default = "default_true")]
    pub enable: bool,
    /// Value for value-style steps (power plan alias, memory mode).
    #[serde(default)]
    pub value: String,
    /// Risk declared by the step. Re-checked against the Safety Engine at apply
    /// time; a declared `Safe` never bypasses the engine's own classification.
    pub risk: RiskLevel,
    /// Why this step is in the profile.
    #[serde(default)]
    pub reason: String,
}

fn default_true() -> bool {
    true
}

impl ProfileStep {
    pub fn new(kind: StepKind, target: &str, label: &str, risk: RiskLevel) -> Self {
        Self {
            kind,
            target: target.to_string(),
            label: label.to_string(),
            enable: true,
            value: String::new(),
            risk,
            reason: String::new(),
        }
    }

    pub fn with_reason(mut self, reason: &str) -> Self {
        self.reason = reason.to_string();
        self
    }

    pub fn with_value(mut self, value: &str) -> Self {
        self.value = value.to_string();
        self
    }

    pub fn disabling(mut self) -> Self {
        self.enable = false;
        self
    }
}

/// Where a profile came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProfileOrigin {
    /// Shipped with Wino. Cannot be deleted; can be duplicated.
    BuiltIn,
    /// Created by the user, stored under the Wino config directory.
    User,
}

/// A named, ordered collection of steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// Stable slug, unique across built-in and user profiles.
    pub id: String,
    pub name: String,
    pub description: String,
    pub origin: ProfileOrigin,
    pub steps: Vec<ProfileStep>,
    /// Whether this profile is applied by default when no name is given.
    #[serde(default)]
    pub is_default: bool,
    /// Last time this profile was applied, for the "Last Applied" badge.
    #[serde(default)]
    pub last_applied: Option<String>,
}

impl Profile {
    pub fn new(id: &str, name: &str, description: &str, origin: ProfileOrigin) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            origin,
            steps: Vec::new(),
            is_default: false,
            last_applied: None,
        }
    }

    pub fn with_step(mut self, step: ProfileStep) -> Self {
        self.steps.push(step);
        self
    }

    pub fn is_builtin(&self) -> bool {
        self.origin == ProfileOrigin::BuiltIn
    }

    /// Highest risk level among the steps, used for the card badge.
    pub fn max_risk(&self) -> RiskLevel {
        self.steps
            .iter()
            .map(|s| s.risk)
            .max()
            .unwrap_or(RiskLevel::Safe)
    }

    /// Steps that the Safety Engine would hard-block (Critical).
    pub fn blocked_steps(&self) -> Vec<&ProfileStep> {
        self.steps
            .iter()
            .filter(|s| s.risk.is_hard_blocked())
            .collect()
    }

    /// Steps grouped by kind, preserving order within a group.
    pub fn steps_by_kind(&self) -> Vec<(StepKind, Vec<&ProfileStep>)> {
        let mut out: Vec<(StepKind, Vec<&ProfileStep>)> = Vec::new();
        for step in &self.steps {
            match out.iter_mut().find(|(k, _)| *k == step.kind) {
                Some((_, list)) => list.push(step),
                None => out.push((step.kind, vec![step])),
            }
        }
        out
    }

    /// A copy with a new identity, for "Duplicate".
    pub fn duplicate_as(&self, new_id: &str, new_name: &str) -> Profile {
        Profile {
            id: new_id.to_string(),
            name: new_name.to_string(),
            description: self.description.clone(),
            origin: ProfileOrigin::User,
            steps: self.steps.clone(),
            is_default: false,
            last_applied: None,
        }
    }

    /// Validate before saving: ids, names, and step targets must be present.
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("Profile id must not be empty.".to_string());
        }
        if self.name.trim().is_empty() {
            return Err("Profile name must not be empty.".to_string());
        }
        for (idx, step) in self.steps.iter().enumerate() {
            if step.target.trim().is_empty() {
                return Err(format!("Step {} has an empty target.", idx + 1));
            }
            if step.label.trim().is_empty() {
                return Err(format!("Step {} has an empty label.", idx + 1));
            }
        }
        Ok(())
    }
}

/// Outcome of one applied step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepOutcome {
    pub label: String,
    pub kind: StepKind,
    pub success: bool,
    /// Set when the step was skipped rather than attempted (blocked by the
    /// Safety Engine, or missing administrator privileges).
    pub skipped_reason: Option<String>,
}

/// Result of applying a profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileApplyReport {
    pub profile_id: String,
    pub profile_name: String,
    pub outcomes: Vec<StepOutcome>,
    pub message: String,
    pub snapshot_id: Option<String>,
}

impl ProfileApplyReport {
    pub fn applied_count(&self) -> usize {
        self.outcomes.iter().filter(|o| o.success).count()
    }

    pub fn failed_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|o| !o.success && o.skipped_reason.is_none())
            .count()
    }

    pub fn skipped_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|o| o.skipped_reason.is_some())
            .count()
    }

    /// True when at least one step failed. The UI must never present a partial
    /// application as complete success.
    pub fn is_partial(&self) -> bool {
        self.failed_count() > 0 || self.skipped_count() > 0
    }

    /// Human summary that distinguishes complete, partial, and total failure.
    pub fn summary(&self) -> String {
        let applied = self.applied_count();
        let failed = self.failed_count();
        let skipped = self.skipped_count();
        if failed == 0 && skipped == 0 {
            format!(
                "Profile '{}' applied ({} steps).",
                self.profile_name, applied
            )
        } else {
            format!(
                "Profile '{}': {} applied, {} failed, {} skipped.",
                self.profile_name, applied, failed, skipped
            )
        }
    }
}

/// Ids of the built-in profiles, in display order.
pub const BUILTIN_IDS: &[&str] = &[
    "balanced",
    "gaming",
    "performance",
    "battery_saver",
    "privacy",
    "low_ram",
];

/// Turn a display name into a filesystem-safe slug.
pub fn slugify(name: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('_');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('_').to_string();
    if trimmed.is_empty() {
        "profile".to_string()
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(kind: StepKind, target: &str, risk: RiskLevel) -> ProfileStep {
        ProfileStep::new(kind, target, target, risk)
    }

    #[test]
    fn max_risk_and_blocked_steps_track_declared_levels() {
        let profile = Profile::new("p", "P", "", ProfileOrigin::User)
            .with_step(step(StepKind::Registry, "a", RiskLevel::Safe))
            .with_step(step(StepKind::Service, "b", RiskLevel::High));
        assert_eq!(profile.max_risk(), RiskLevel::High);
        assert!(profile.blocked_steps().is_empty());

        let with_critical = profile.with_step(step(StepKind::Registry, "c", RiskLevel::Critical));
        assert_eq!(with_critical.blocked_steps().len(), 1);
        assert_eq!(with_critical.max_risk(), RiskLevel::Critical);
    }

    #[test]
    fn steps_by_kind_groups_without_reordering() {
        let profile = Profile::new("p", "P", "", ProfileOrigin::User)
            .with_step(step(StepKind::Registry, "r1", RiskLevel::Safe))
            .with_step(step(StepKind::Service, "s1", RiskLevel::Safe))
            .with_step(step(StepKind::Registry, "r2", RiskLevel::Low));

        let grouped = profile.steps_by_kind();
        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped[0].0, StepKind::Registry);
        assert_eq!(grouped[0].1.len(), 2);
        assert_eq!(grouped[0].1[0].target, "r1");
        assert_eq!(grouped[0].1[1].target, "r2");
        assert_eq!(grouped[1].0, StepKind::Service);
    }

    #[test]
    fn duplicate_becomes_a_user_profile_with_a_new_identity() {
        let builtin = Profile::new("gaming", "Gaming", "desc", ProfileOrigin::BuiltIn)
            .with_step(step(StepKind::Power, "high_performance", RiskLevel::Low));
        let copy = builtin.duplicate_as("gaming_custom", "Gaming (custom)");

        assert_eq!(copy.id, "gaming_custom");
        assert_eq!(copy.name, "Gaming (custom)");
        assert_eq!(copy.origin, ProfileOrigin::User);
        assert_eq!(copy.steps.len(), 1);
        assert!(!copy.is_default);
        assert!(builtin.is_builtin());
    }

    #[test]
    fn validate_rejects_empty_identity_and_step_fields() {
        let mut profile = Profile::new("ok", "OK", "", ProfileOrigin::User);
        assert!(profile.validate().is_ok());

        profile.name = "   ".to_string();
        assert!(profile.validate().is_err());

        let profile = Profile::new("ok", "OK", "", ProfileOrigin::User).with_step(
            ProfileStep::new(StepKind::Registry, "", "", RiskLevel::Safe),
        );
        assert!(profile.validate().is_err());
    }

    #[test]
    fn apply_report_distinguishes_complete_partial_and_failure() {
        let complete = ProfileApplyReport {
            profile_id: "p".to_string(),
            profile_name: "P".to_string(),
            outcomes: vec![StepOutcome {
                label: "a".to_string(),
                kind: StepKind::Registry,
                success: true,
                skipped_reason: None,
            }],
            message: String::new(),
            snapshot_id: None,
        };
        assert!(!complete.is_partial());
        assert!(complete.summary().contains("applied (1 steps)"));

        let partial = ProfileApplyReport {
            outcomes: vec![
                StepOutcome {
                    label: "a".to_string(),
                    kind: StepKind::Registry,
                    success: true,
                    skipped_reason: None,
                },
                StepOutcome {
                    label: "b".to_string(),
                    kind: StepKind::Service,
                    success: false,
                    skipped_reason: None,
                },
                StepOutcome {
                    label: "c".to_string(),
                    kind: StepKind::Task,
                    success: false,
                    skipped_reason: Some("blocked".to_string()),
                },
            ],
            ..complete.clone()
        };
        assert!(partial.is_partial());
        assert_eq!(partial.applied_count(), 1);
        assert_eq!(partial.failed_count(), 1);
        assert_eq!(partial.skipped_count(), 1);
        assert!(partial.summary().contains("1 applied, 1 failed, 1 skipped"));
    }

    #[test]
    fn slugify_produces_filesystem_safe_ids() {
        assert_eq!(slugify("Gaming (custom)"), "gaming_custom");
        assert_eq!(slugify("  Low   RAM  "), "low_ram");
        assert_eq!(slugify("!!!"), "profile");
        assert_eq!(slugify("Already_Slug"), "already_slug");
    }

    #[test]
    fn builtin_profile_round_trips_through_toml() {
        let profile = Profile::new("balanced", "Balanced", "desc", ProfileOrigin::BuiltIn)
            .with_step(step(
                StepKind::Registry,
                "disable_bing_search_start",
                RiskLevel::Safe,
            ));
        let text = toml::to_string_pretty(&profile).expect("serialize profile");
        let back: Profile = toml::from_str(&text).expect("deserialize profile");
        assert_eq!(back.id, profile.id);
        assert_eq!(back.steps.len(), 1);
        assert_eq!(back.steps[0].kind, StepKind::Registry);
        assert!(back.steps[0].enable);
    }
}
