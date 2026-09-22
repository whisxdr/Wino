use serde::{Deserialize, Serialize};

/// Risk classification shared by every mutating subsystem in Wino.
///
/// Ordering is meaningful: `Safe < Low < Medium < High < Critical`. Batch
/// operations compare a rule's level against the active preset ceiling with
/// `<=`, and `Critical` is never reachable through a preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RiskLevel {
    Safe,
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::Safe => "Safe",
            RiskLevel::Low => "Low",
            RiskLevel::Medium => "Medium",
            RiskLevel::High => "High",
            RiskLevel::Critical => "Critical",
        }
    }

    /// RGB used by the UI badges so every view paints risk identically.
    pub fn color_rgb(&self) -> (u8, u8, u8) {
        match self {
            RiskLevel::Safe => (34, 197, 94),
            RiskLevel::Low => (16, 185, 129),
            RiskLevel::Medium => (234, 179, 8),
            RiskLevel::High => (249, 115, 22),
            RiskLevel::Critical => (239, 68, 68),
        }
    }

    /// Risk levels that always hard-block regardless of user intent.
    pub fn is_hard_blocked(&self) -> bool {
        *self == RiskLevel::Critical
    }

    pub fn is_safe_for_preset(&self, preset: &str) -> bool {
        match preset {
            "Safe" => *self == RiskLevel::Safe,
            "Balanced" => *self <= RiskLevel::Low,
            "Aggressive" => *self <= RiskLevel::Medium,
            "Custom" => *self <= RiskLevel::High, // Critical items are always blocked from batch operations
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyCheckResult {
    pub is_allowed: bool,
    pub risk: RiskLevel,
    pub reason: String,
}

/// Everything the UI must disclose before a user confirms a system change.
///
/// Every destructive or potentially disruptive operation in Wino describes
/// itself with this record so the confirmation dialog shows the same fields
/// for a registry write, a service change, a power setting, or a package
/// removal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationDescriptor {
    /// Stable identifier, usually the rule id.
    pub id: String,
    /// Human-readable operation name.
    pub name: String,
    /// Why the operation exists and what it changes.
    pub reason: String,
    /// Component the operation touches, e.g. "Service: DiagTrack".
    pub component: String,
    pub risk: RiskLevel,
    pub reversible: bool,
    pub requires_admin: bool,
    /// "10", "11", or both. Empty means any supported Windows version.
    pub supported_windows: Vec<String>,
    pub requires_reboot: bool,
    /// Observed state before the change (best effort; empty when unknown).
    pub current_state: String,
    /// State the operation produces.
    pub target_state: String,
}

impl OperationDescriptor {
    /// Minimal descriptor for operations without a declarative rule entry.
    pub fn simple(id: &str, name: &str, component: &str, risk: RiskLevel) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            reason: String::new(),
            component: component.to_string(),
            risk,
            reversible: true,
            requires_admin: false,
            supported_windows: Vec::new(),
            requires_reboot: false,
            current_state: String::new(),
            target_state: String::new(),
        }
    }

    pub fn with_reason(mut self, reason: &str) -> Self {
        self.reason = reason.to_string();
        self
    }

    pub fn with_admin(mut self, requires_admin: bool) -> Self {
        self.requires_admin = requires_admin;
        self
    }

    pub fn with_reversible(mut self, reversible: bool) -> Self {
        self.reversible = reversible;
        self
    }

    pub fn with_reboot(mut self, requires_reboot: bool) -> Self {
        self.requires_reboot = requires_reboot;
        self
    }

    pub fn with_windows(mut self, versions: &[&str]) -> Self {
        self.supported_windows = versions.iter().map(|v| v.to_string()).collect();
        self
    }

    pub fn with_states(mut self, current: &str, target: &str) -> Self {
        self.current_state = current.to_string();
        self.target_state = target.to_string();
        self
    }
}

pub struct SafetyEngine;

impl SafetyEngine {
    pub fn validate_operation(
        name: &str,
        risk: RiskLevel,
        requires_admin: bool,
        supported_windows: &[String],
        sys_info: &crate::core::system::SystemInfo,
    ) -> SafetyCheckResult {
        // 1. Critical risk level check
        if risk.is_hard_blocked() {
            return SafetyCheckResult {
                is_allowed: false,
                risk,
                reason: format!(
                    "Operation '{}' touches Windows core stability. Wino blocks it.",
                    name
                ),
            };
        }

        // 2. Windows version support check
        let win_ver_str = if sys_info.is_windows_11 { "11" } else { "10" };
        if !supported_windows.is_empty() && !supported_windows.iter().any(|v| v == win_ver_str) {
            return SafetyCheckResult {
                is_allowed: false,
                risk,
                reason: format!(
                    "Operation '{}' is not supported on Windows {}.",
                    name, win_ver_str
                ),
            };
        }

        // 3. Admin privilege check
        if requires_admin && !sys_info.is_admin {
            return SafetyCheckResult {
                is_allowed: false,
                risk,
                reason: format!("Operation '{}' requires Administrator privileges.", name),
            };
        }

        SafetyCheckResult {
            is_allowed: true,
            risk,
            reason: "Safe to proceed.".to_string(),
        }
    }

    /// Validate a full [`OperationDescriptor`] against the live system.
    ///
    /// Same gates as [`Self::validate_operation`]. Callers must treat
    /// `is_allowed == false` as a hard stop and surface `reason` verbatim.
    pub fn validate_descriptor(
        desc: &OperationDescriptor,
        sys_info: &crate::core::system::SystemInfo,
    ) -> SafetyCheckResult {
        Self::validate_operation(
            &desc.name,
            desc.risk,
            desc.requires_admin,
            &desc.supported_windows,
            sys_info,
        )
    }

    /// Split a batch of descriptors into (allowed, blocked) for `preset`.
    ///
    /// Returns the blocked descriptors too, so the UI can explain exactly which
    /// rules were skipped instead of silently dropping them.
    pub fn filter_for_preset<'a>(
        descs: &'a [OperationDescriptor],
        preset: &str,
    ) -> (Vec<&'a OperationDescriptor>, Vec<&'a OperationDescriptor>) {
        descs
            .iter()
            .partition(|d| d.risk.is_safe_for_preset(preset))
    }
}
