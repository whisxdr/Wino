use crate::core::safety::RiskLevel;
use crate::debloat::rules::RegistryRuleItem;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub risk: RiskLevel,
    pub reversible: bool,
    pub requires_admin: bool,
    pub supported_windows: Vec<String>,
    pub reason: String,
    pub impact: String,
    pub registry_keys: Vec<RegistryRuleItem>,
}

pub fn load_privacy_rules() -> Vec<PrivacyRule> {
    const PRIVACY_JSON: &str = include_str!("../../data/privacy_rules.json");
    serde_json::from_str(PRIVACY_JSON).unwrap_or_default()
}
