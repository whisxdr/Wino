use crate::core::safety::RiskLevel;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryRuleItem {
    pub hive: String,
    pub path: String,
    pub value_name: String,
    pub value_type: String,
    pub value_data: u32,
    pub restore_data: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebloatRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub preset: String, // "Safe", "Balanced", "Aggressive", "Custom"
    pub risk: RiskLevel,
    pub reversible: bool,
    pub requires_admin: bool,
    pub supported_windows: Vec<String>,
    pub reason: String,
    pub estimated_benefit: String,
    #[serde(default)]
    pub registry_keys: Vec<RegistryRuleItem>,
    #[serde(default)]
    pub package_names: Vec<String>,
    #[serde(default)]
    pub services: Vec<String>,
}

pub fn load_default_rules() -> Vec<DebloatRule> {
    const DEFAULT_RULES_JSON: &str = include_str!("../../data/debloat_rules.json");
    serde_json::from_str(DEFAULT_RULES_JSON).unwrap_or_default()
}
