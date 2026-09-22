//! Power Manager data model.
//!
//! Plans and settings are addressed by GUID because that is how the Windows
//! power API identifies them. Friendly names are read from the API and are
//! localized by Windows itself, so they are never hard-coded here.

use serde::{Deserialize, Serialize};

/// Well-known Windows power scheme GUIDs.
pub mod scheme_guids {
    /// Balanced (`381b4222-f694-41f0-9685-ff5bb260df2e`).
    pub const BALANCED: &str = "381B4222-F694-41F0-9685-FF5BB260DF2E";
    /// High Performance (`8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c`).
    pub const HIGH_PERFORMANCE: &str = "8C5E7FDA-E8BF-4A96-9A85-A6E23A8C635C";
    /// Power Saver (`a1841308-3541-4fab-bc81-f71556f20b4a`).
    pub const POWER_SAVER: &str = "A1841308-3541-4FAB-BC81-F71556F20B4A";
    /// Ultimate Performance. Not present on every edition, and only created by
    /// `powercfg -duplicatescheme e9a42b02-d5df-448d-aa00-03f14749eb61`.
    pub const ULTIMATE_PERFORMANCE: &str = "E9A42B02-D5DF-448D-AA00-03F14749EB61";
}

/// One power scheme reported by the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerPlan {
    /// Uppercase GUID without braces.
    pub guid: String,
    /// Name as Windows reports it (already localized by Windows).
    pub name: String,
    /// True for the currently active scheme.
    pub is_active: bool,
    /// Which well-known plan this is, when it is one.
    pub kind: PowerPlanKind,
}

impl PowerPlan {
    /// i18n key for the plan's category label. The concrete name always comes
    /// from Windows so a localized install shows its own wording.
    pub fn kind_i18n_key(&self) -> &'static str {
        self.kind.i18n_key()
    }

    /// Whether Wino offers this plan in the quick-apply list.
    pub fn is_offered(&self) -> bool {
        matches!(
            self.kind,
            PowerPlanKind::Balanced
                | PowerPlanKind::HighPerformance
                | PowerPlanKind::Ultimate
                | PowerPlanKind::PowerSaver
        )
    }
}

/// Classification of a power scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerPlanKind {
    Balanced,
    HighPerformance,
    Ultimate,
    PowerSaver,
    Custom,
}

impl PowerPlanKind {
    pub fn i18n_key(&self) -> &'static str {
        match self {
            PowerPlanKind::Balanced => "power.plan_balanced",
            PowerPlanKind::HighPerformance => "power.plan_high_performance",
            PowerPlanKind::Ultimate => "power.plan_ultimate",
            PowerPlanKind::PowerSaver => "power.plan_power_saver",
            PowerPlanKind::Custom => "power.plan_custom",
        }
    }

    /// Classify by GUID. GUID comparison is case-insensitive because the API
    /// returns mixed-case strings across Windows versions.
    pub fn from_guid(guid: &str) -> Self {
        let upper = guid.to_uppercase();
        let upper = upper.trim_matches(|c| c == '{' || c == '}');
        match upper {
            scheme_guids::BALANCED => PowerPlanKind::Balanced,
            scheme_guids::HIGH_PERFORMANCE => PowerPlanKind::HighPerformance,
            scheme_guids::ULTIMATE_PERFORMANCE => PowerPlanKind::Ultimate,
            scheme_guids::POWER_SAVER => PowerPlanKind::PowerSaver,
            _ => PowerPlanKind::Custom,
        }
    }
}

/// One advanced power setting exposed by the active scheme.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerSetting {
    /// Stable Wino id, e.g. "processor_min".
    pub id: String,
    /// Display name (from Windows when available, else the curated label).
    pub name: String,
    pub subgroup_guid: String,
    pub setting_guid: String,
    /// Value as currently configured on AC power. `None` when the setting is
    /// not exposed by the active scheme on this system.
    pub ac_value: Option<u32>,
    pub dc_value: Option<u32>,
    /// Smallest allowed value reported by the API.
    pub min_value: u32,
    /// Largest allowed value reported by the API.
    pub max_value: u32,
    /// Unit label for the value ("%", "seconds", "minutes", ...).
    pub unit: String,
    /// True when the value is an enumerated choice rather than a range.
    pub is_enumerated: bool,
    /// Friendly names for enumerated values, index-aligned.
    pub enum_labels: Vec<String>,
    /// Human note explaining the effect of the setting.
    pub note: String,
}

impl PowerSetting {
    /// Whether the setting could be read from the active scheme.
    pub fn is_available(&self) -> bool {
        self.ac_value.is_some() || self.dc_value.is_some()
    }

    /// Render one value using the enum labels when the setting is enumerated.
    pub fn format_value(&self, value: Option<u32>) -> String {
        let Some(v) = value else {
            return "—".to_string();
        };
        if self.is_enumerated {
            return self
                .enum_labels
                .get(v as usize)
                .cloned()
                .unwrap_or_else(|| v.to_string());
        }
        if self.unit.is_empty() {
            v.to_string()
        } else {
            format!("{} {}", v, self.unit)
        }
    }

    /// Clamp a requested value into the range the API reports.
    pub fn clamp(&self, value: u32) -> u32 {
        value.clamp(self.min_value, self.max_value)
    }
}

/// Curated ids for the settings Wino exposes, in display order.
pub const EXPOSED_SETTING_IDS: &[&str] = &[
    "processor_min",
    "processor_max",
    "processor_boost",
    "sleep_timeout",
    "display_timeout",
    "usb_suspend",
    "pcie_link",
    "disk_timeout",
];

/// Human label key for a setting id.
pub fn setting_label_key(id: &str) -> &'static str {
    match id {
        "processor_min" => "power.setting_processor_min",
        "processor_max" => "power.setting_processor_max",
        "processor_boost" => "power.setting_boost_mode",
        "sleep_timeout" => "power.setting_sleep_timeout",
        "display_timeout" => "power.setting_display_timeout",
        "usb_suspend" => "power.setting_usb_suspend",
        "pcie_link" => "power.setting_pcie_link",
        "disk_timeout" => "power.setting_disk_timeout",
        _ => "power.setting_unavailable",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guid_classification_is_case_and_brace_insensitive() {
        assert_eq!(
            PowerPlanKind::from_guid("381b4222-f694-41f0-9685-ff5bb260df2e"),
            PowerPlanKind::Balanced
        );
        assert_eq!(
            PowerPlanKind::from_guid("{8C5E7FDA-E8BF-4A96-9A85-A6E23A8C635C}"),
            PowerPlanKind::HighPerformance
        );
        assert_eq!(
            PowerPlanKind::from_guid("e9a42b02-d5df-448d-aa00-03f14749eb61"),
            PowerPlanKind::Ultimate
        );
        assert_eq!(
            PowerPlanKind::from_guid("a1841308-3541-4fab-bc81-f71556f20b4a"),
            PowerPlanKind::PowerSaver
        );
        assert_eq!(
            PowerPlanKind::from_guid("11111111-2222-3333-4444-555555555555"),
            PowerPlanKind::Custom
        );
    }

    #[test]
    fn plan_offer_list_excludes_custom_schemes() {
        let custom = PowerPlan {
            guid: "X".to_string(),
            name: "Vendor Turbo".to_string(),
            is_active: false,
            kind: PowerPlanKind::Custom,
        };
        assert!(!custom.is_offered());

        let balanced = PowerPlan {
            guid: scheme_guids::BALANCED.to_string(),
            name: "Balanced".to_string(),
            is_active: true,
            kind: PowerPlanKind::Balanced,
        };
        assert!(balanced.is_offered());
    }

    #[test]
    fn enumerated_setting_renders_label_and_plain_setting_renders_unit() {
        let enumerated = PowerSetting {
            id: "processor_boost".to_string(),
            name: "Processor boost".to_string(),
            subgroup_guid: "SUB".to_string(),
            setting_guid: "SET".to_string(),
            ac_value: Some(2),
            dc_value: Some(0),
            min_value: 0,
            max_value: 4,
            unit: String::new(),
            is_enumerated: true,
            enum_labels: vec![
                "Disabled".to_string(),
                "Enabled".to_string(),
                "Aggressive".to_string(),
            ],
            note: String::new(),
        };
        assert_eq!(enumerated.format_value(Some(2)), "Aggressive");
        // Out-of-range index falls back to the raw number rather than panicking.
        assert_eq!(enumerated.format_value(Some(9)), "9");
        assert_eq!(enumerated.format_value(None), "—");

        let ranged = PowerSetting {
            is_enumerated: false,
            unit: "%".to_string(),
            ac_value: Some(100),
            ..enumerated.clone()
        };
        assert_eq!(ranged.format_value(Some(100)), "100 %");
        assert!(ranged.is_available());
    }

    #[test]
    fn clamp_keeps_values_inside_reported_range() {
        let setting = PowerSetting {
            id: "processor_max".to_string(),
            name: "Processor max".to_string(),
            subgroup_guid: "SUB".to_string(),
            setting_guid: "SET".to_string(),
            ac_value: Some(100),
            dc_value: Some(100),
            min_value: 5,
            max_value: 100,
            unit: "%".to_string(),
            is_enumerated: false,
            enum_labels: Vec::new(),
            note: String::new(),
        };
        assert_eq!(setting.clamp(0), 5);
        assert_eq!(setting.clamp(50), 50);
        assert_eq!(setting.clamp(500), 100);
    }

    #[test]
    fn unavailable_setting_reports_not_available() {
        let setting = PowerSetting {
            id: "pcie_link".to_string(),
            name: "PCI Express".to_string(),
            subgroup_guid: "SUB".to_string(),
            setting_guid: "SET".to_string(),
            ac_value: None,
            dc_value: None,
            min_value: 0,
            max_value: 2,
            unit: String::new(),
            is_enumerated: true,
            enum_labels: Vec::new(),
            note: String::new(),
        };
        assert!(!setting.is_available());
    }
}
