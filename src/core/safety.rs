use serde::{Deserialize, Serialize};

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
        if risk == RiskLevel::Critical {
            return SafetyCheckResult {
                is_allowed: false,
                risk,
                reason: format!("Operation '{}' is marked as CRITICAL to Windows core stability and cannot be modified automatically.", name),
            };
        }

        // 2. Windows version support check
        let win_ver_str = if sys_info.is_windows_11 { "11" } else { "10" };
        if !supported_windows.is_empty() && !supported_windows.iter().any(|v| v == win_ver_str) {
            return SafetyCheckResult {
                is_allowed: false,
                risk,
                reason: format!("Operation '{}' is not supported on Windows {}.", name, win_ver_str),
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
}
