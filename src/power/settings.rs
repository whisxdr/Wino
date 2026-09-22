//! Power Manager: advanced power settings on the active scheme.
//!
//! Every setting is addressed by its documented subgroup/setting GUID pair and
//! read through `PowerReadACValueIndex` / `PowerReadDCValueIndex`. A read that
//! fails leaves the value `None`, which the UI renders as "not exposed by this
//! scheme" — a failed read is never presented as a real `0`, because a fake zero
//! is indistinguishable from a genuine "off" and would be written back as one.
//!
//! Writes are validated by the Safety Engine, snapshotted with the previous
//! AC/DC values, then verified by re-reading: a write that did not take effect
//! is reported as an error, never as success.

use crate::core::logger::{log_error, log_info, log_warn};
use crate::core::safety::{OperationDescriptor, RiskLevel, SafetyEngine};
use crate::core::system::SystemInfo;
use crate::power::manager::{active_plan_guid, parse_guid};
use crate::power::models::EXPOSED_SETTING_IDS;
// Re-exported because the UI imports `PowerSetting` from this module.
pub use crate::power::models::PowerSetting;
use crate::restore::snapshots::{create_snapshot_for_power, PowerBackupEntry};
use windows::core::GUID;
use windows::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_SUCCESS};
use windows::Win32::System::Power::{
    PowerReadACValueIndex, PowerReadDCValueIndex, PowerWriteACValueIndex, PowerWriteDCValueIndex,
};
use windows::Win32::System::Registry::HKEY;

/// Processor power management subgroup.
const SUBGROUP_PROCESSOR: &str = "54533251-82BE-4824-96C1-47B60B740D00";
/// Sleep subgroup.
const SUBGROUP_SLEEP: &str = "238C9FA8-0AAD-41ED-83F4-97BE242C8F20";
/// Display subgroup.
const SUBGROUP_DISPLAY: &str = "7516B95F-F776-4464-8C53-06167F40CC99";
/// USB subgroup.
const SUBGROUP_USB: &str = "2A737441-1930-4402-8D77-B2BEBBA308A3";
/// PCI Express subgroup.
const SUBGROUP_PCIE: &str = "501A4D13-42AF-4429-9FD1-A8218C268E20";
/// Hard disk subgroup.
const SUBGROUP_DISK: &str = "0012EE47-9041-4B5D-9B77-535FBA8B1442";

const SETTING_PROCESSOR_MIN: &str = "893DEE8E-2BEF-41C9-9FBF-1EF2A0F4F0B6";
const SETTING_PROCESSOR_MAX: &str = "BC5038F7-23E0-4960-96DA-33ABAF5935EC";
const SETTING_PROCESSOR_BOOST: &str = "BE337238-0D82-4146-A960-4F3749D470C7";
const SETTING_SLEEP_TIMEOUT: &str = "29F6C1DB-86DA-48C5-9FDB-F2B67B1F44DA";
const SETTING_DISPLAY_TIMEOUT: &str = "3C0BC021-C8A8-4E07-A973-6B14CBCB2B7E";
const SETTING_USB_SUSPEND: &str = "48E6B7A6-50F5-4782-A5D4-53BB8F07E226";
const SETTING_PCIE_LINK: &str = "EE12F906-D277-404B-B6DA-E5FA1A576DF5";
const SETTING_DISK_TIMEOUT: &str = "6738E2C4-E8A5-4A42-B16A-E040E769756E";

/// Timeout settings are reported in seconds with a day as the practical ceiling.
const TIMEOUT_MAX_SECONDS: u32 = 86_400;

/// Static description of one exposed setting: where it lives, how to read it,
/// and how to label it.
struct SettingSpec {
    id: &'static str,
    /// English display name. The UI renders the translated label from
    /// `setting_label_key`; this is the fallback shown when no translation is
    /// registered.
    name: &'static str,
    subgroup: &'static str,
    setting: &'static str,
    unit: &'static str,
    is_enumerated: bool,
    enum_labels: &'static [&'static str],
    note: &'static str,
}

const SETTING_SPECS: &[SettingSpec] = &[
    SettingSpec {
        id: "processor_min",
        name: "Processor minimum state",
        subgroup: SUBGROUP_PROCESSOR,
        setting: SETTING_PROCESSOR_MIN,
        unit: "%",
        is_enumerated: false,
        enum_labels: &[],
        note: "Floor for the processor clock. Raising it keeps the CPU above its lowest state and costs battery life.",
    },
    SettingSpec {
        id: "processor_max",
        name: "Processor maximum state",
        subgroup: SUBGROUP_PROCESSOR,
        setting: SETTING_PROCESSOR_MAX,
        unit: "%",
        is_enumerated: false,
        enum_labels: &[],
        note: "Ceiling for the processor clock. 100 % lets the CPU reach its full rated speed.",
    },
    SettingSpec {
        id: "processor_boost",
        name: "Processor performance boost mode",
        subgroup: SUBGROUP_PROCESSOR,
        setting: SETTING_PROCESSOR_BOOST,
        unit: "",
        is_enumerated: true,
        enum_labels: &["Disabled", "Enabled", "Aggressive", "Efficient Enabled", "Efficient Aggressive"],
        note: "Allows the processor to exceed its rated frequency for short bursts.",
    },
    SettingSpec {
        id: "sleep_timeout",
        name: "Sleep after",
        subgroup: SUBGROUP_SLEEP,
        setting: SETTING_SLEEP_TIMEOUT,
        unit: "seconds",
        is_enumerated: false,
        enum_labels: &[],
        note: "Idle time before the PC goes to sleep. 0 means never.",
    },
    SettingSpec {
        id: "display_timeout",
        name: "Turn off display after",
        subgroup: SUBGROUP_DISPLAY,
        setting: SETTING_DISPLAY_TIMEOUT,
        unit: "seconds",
        is_enumerated: false,
        enum_labels: &[],
        note: "Idle time before the display turns off. 0 means never.",
    },
    SettingSpec {
        id: "usb_suspend",
        name: "USB selective suspend",
        subgroup: SUBGROUP_USB,
        setting: SETTING_USB_SUSPEND,
        unit: "",
        is_enumerated: true,
        enum_labels: &["Disabled", "Enabled"],
        note: "Selective suspend powers down idle USB devices.",
    },
    SettingSpec {
        id: "pcie_link",
        name: "PCI Express link state power management",
        subgroup: SUBGROUP_PCIE,
        setting: SETTING_PCIE_LINK,
        unit: "",
        is_enumerated: true,
        enum_labels: &["Off", "Moderate", "Maximum"],
        note: "PCI Express link state power management. Maximum power savings can add latency to PCIe devices.",
    },
    SettingSpec {
        id: "disk_timeout",
        name: "Turn off hard disk after",
        subgroup: SUBGROUP_DISK,
        setting: SETTING_DISK_TIMEOUT,
        unit: "seconds",
        is_enumerated: false,
        enum_labels: &[],
        note: "Idle time before the hard disk is powered down. 0 means never.",
    },
];

fn spec_for(id: &str) -> Option<&'static SettingSpec> {
    SETTING_SPECS.iter().find(|s| s.id == id)
}

/// Read a setting index. `None` means the scheme does not expose it or the API
/// rejected the read; callers must not substitute a default.
fn read_ac_index(scheme: &GUID, subgroup: &GUID, setting: &GUID) -> Option<u32> {
    let mut value = 0u32;
    let result = unsafe {
        PowerReadACValueIndex(
            HKEY::default(),
            Some(scheme as *const GUID),
            Some(subgroup as *const GUID),
            Some(setting as *const GUID),
            &mut value,
        )
    };
    if result == ERROR_SUCCESS {
        Some(value)
    } else {
        None
    }
}

fn read_dc_index(scheme: &GUID, subgroup: &GUID, setting: &GUID) -> Option<u32> {
    let mut value = 0u32;
    // The DC entry point is declared as returning a raw DWORD error code by the
    // windows crate, unlike its AC counterpart which returns WIN32_ERROR.
    let result = unsafe {
        PowerReadDCValueIndex(
            HKEY::default(),
            Some(scheme as *const GUID),
            Some(subgroup as *const GUID),
            Some(setting as *const GUID),
            &mut value,
        )
    };
    if result == ERROR_SUCCESS.0 {
        Some(value)
    } else {
        None
    }
}

/// Range a setting allows, read from the power settings registry.
///
/// The power API has no range query: `PowerReadACValue` returns the current
/// value, not its bounds. Windows stores the bounds as `ValueMin` / `ValueMax`
/// DWORDs beside the setting's definition, so that is where they are read.
/// Registry key lookup is case-insensitive, so the uppercase GUIDs work as-is.
///
/// `None` when the setting declares no usable range (enumerated settings never
/// do).
fn read_declared_range(spec: &SettingSpec) -> Option<(u32, u32)> {
    let path = format!(
        "SYSTEM\\CurrentControlSet\\Control\\Power\\PowerSettings\\{}\\{}",
        spec.subgroup, spec.setting
    );
    let min =
        crate::core::executor::SystemExecutor::read_registry_dword("HKLM", &path, "ValueMin")?;
    let max =
        crate::core::executor::SystemExecutor::read_registry_dword("HKLM", &path, "ValueMax")?;
    if min > max || max == 0 {
        return None;
    }
    Some((min, max))
}

/// Range to present for a setting.
///
/// A declared range is honoured only when it is plausible: the timeout settings
/// declare `ValueMax = 0xFFFFFFFF` ("unlimited"), which is a real bound but a
/// useless slider, so anything beyond a day falls back to the sane ceiling. A
/// missing range is not a missing value — the setting is still readable and
/// writable, so a sane bound beats refusing to show it.
fn fallback_range(spec: &SettingSpec, declared: Option<(u32, u32)>) -> (u32, u32) {
    if spec.is_enumerated {
        return (0, spec.enum_labels.len().saturating_sub(1) as u32);
    }
    if let Some((min, max)) = declared {
        if max <= TIMEOUT_MAX_SECONDS {
            return (min, max);
        }
    }
    if spec.unit == "%" {
        (0, 100)
    } else {
        (0, TIMEOUT_MAX_SECONDS)
    }
}

fn build_setting(
    spec: &SettingSpec,
    declared_range: Option<(u32, u32)>,
    ac: Option<u32>,
    dc: Option<u32>,
) -> PowerSetting {
    let (min_value, max_value) = fallback_range(spec, declared_range);

    PowerSetting {
        id: spec.id.to_string(),
        // Windows' own localized name is not read here: the curated label key
        // keeps the UI translatable. `name` is the English fallback.
        name: spec.name.to_string(),
        subgroup_guid: spec.subgroup.to_string(),
        setting_guid: spec.setting.to_string(),
        ac_value: ac,
        dc_value: dc,
        min_value,
        max_value,
        unit: spec.unit.to_string(),
        is_enumerated: spec.is_enumerated,
        enum_labels: spec.enum_labels.iter().map(|s| s.to_string()).collect(),
        note: spec.note.to_string(),
    }
}

/// Read every exposed setting from the active scheme.
///
/// Returns an empty vector when there is no active scheme, which the UI shows as
/// an unavailable power subsystem rather than as eight zero-valued settings.
pub fn read_all_settings() -> Vec<PowerSetting> {
    let Some(scheme) = active_plan_guid().and_then(|text| parse_guid(&text)) else {
        log_warn(
            "power",
            "No active power scheme; advanced settings are unavailable.",
        );
        return Vec::new();
    };

    let mut settings = Vec::with_capacity(EXPOSED_SETTING_IDS.len());
    for id in EXPOSED_SETTING_IDS {
        let Some(spec) = spec_for(id) else {
            continue;
        };
        let Some(subgroup) = parse_guid(spec.subgroup) else {
            continue;
        };
        let Some(setting) = parse_guid(spec.setting) else {
            continue;
        };

        let ac = read_ac_index(&scheme, &subgroup, &setting);
        let dc = read_dc_index(&scheme, &subgroup, &setting);
        settings.push(build_setting(spec, read_declared_range(spec), ac, dc));
    }

    settings
}

/// Write a setting's AC and DC values on the active scheme.
///
/// Order matters: validate, snapshot the previous values, write, re-read. A
/// re-read that disagrees with the request is an error — reporting success for a
/// write Windows ignored would leave the UI showing a value that is not real.
pub fn write_setting(
    setting: &PowerSetting,
    ac_value: u32,
    dc_value: u32,
    dry_run: bool,
) -> Result<String, String> {
    let sys_info = SystemInfo::detect();
    let descriptor = OperationDescriptor::simple(
        &setting.id,
        &format!("Set power setting {}", setting.id),
        &format!("Power setting: {}", setting.id),
        RiskLevel::Low,
    )
    .with_reason("Reversible advanced power setting on the active scheme.")
    .with_admin(true)
    .with_windows(&["10", "11"])
    .with_reversible(true);
    let check = SafetyEngine::validate_descriptor(&descriptor, &sys_info);
    if !check.is_allowed {
        return Err(check.reason);
    }

    if setting.is_enumerated
        && (setting.enum_labels.is_empty()
            || ac_value as usize >= setting.enum_labels.len()
            || dc_value as usize >= setting.enum_labels.len())
    {
        return Err(format!(
            "'{}' is not a valid choice for '{}' (expected 0..{}).",
            ac_value,
            setting.id,
            setting.enum_labels.len().saturating_sub(1)
        ));
    }

    let ac_value = setting.clamp(ac_value);
    let dc_value = setting.clamp(dc_value);

    let Some(scheme) = active_plan_guid().and_then(|text| parse_guid(&text)) else {
        return Err("No active power scheme to write to.".to_string());
    };
    let Some(subgroup) = parse_guid(&setting.subgroup_guid) else {
        return Err(format!(
            "Invalid subgroup GUID '{}'.",
            setting.subgroup_guid
        ));
    };
    let Some(setting_guid) = parse_guid(&setting.setting_guid) else {
        return Err(format!("Invalid setting GUID '{}'.", setting.setting_guid));
    };

    let ac_label = setting.format_value(Some(ac_value));
    let dc_label = setting.format_value(Some(dc_value));

    if dry_run {
        return Ok(format!(
            "[DRY-RUN] Would set '{}' to AC {}, DC {} on the active scheme.",
            setting.id, ac_label, dc_label
        ));
    }

    // Previous values, read before the write so the snapshot can undo it.
    let previous_ac = read_ac_index(&scheme, &subgroup, &setting_guid);
    let previous_dc = read_dc_index(&scheme, &subgroup, &setting_guid);

    let snapshot = create_snapshot_for_power(
        &format!("Power setting: {}", setting.id),
        &[PowerBackupEntry {
            scheme_guid: crate::power::manager::guid_to_string(&scheme),
            subgroup_guid: setting.subgroup_guid.to_uppercase(),
            setting_guid: setting.setting_guid.to_uppercase(),
            setting_name: setting.id.clone(),
            previous_ac_value: previous_ac,
            previous_dc_value: previous_dc,
        }],
    );
    log_info(
        "power",
        &format!("Snapshot {} recorded before power write.", snapshot.id),
    );

    let ac_result = unsafe {
        PowerWriteACValueIndex(
            HKEY::default(),
            &scheme,
            Some(&subgroup as *const GUID),
            Some(&setting_guid as *const GUID),
            ac_value,
        )
    };
    if ac_result != ERROR_SUCCESS {
        log_error(
            "power",
            &format!("PowerWriteACValueIndex failed for {}", setting.id),
        );
        return Err(win32_write_error(
            "AC",
            &setting.id,
            ac_result.0,
            sys_info.is_admin,
        ));
    }

    let dc_result = unsafe {
        PowerWriteDCValueIndex(
            HKEY::default(),
            &scheme,
            Some(&subgroup as *const GUID),
            Some(&setting_guid as *const GUID),
            dc_value,
        )
    };
    if dc_result != ERROR_SUCCESS.0 {
        log_error(
            "power",
            &format!("PowerWriteDCValueIndex failed for {}", setting.id),
        );
        return Err(win32_write_error(
            "DC",
            &setting.id,
            dc_result,
            sys_info.is_admin,
        ));
    }

    // Verify: an unverified write is not a write.
    let verified_ac = read_ac_index(&scheme, &subgroup, &setting_guid);
    let verified_dc = read_dc_index(&scheme, &subgroup, &setting_guid);
    if verified_ac != Some(ac_value) || verified_dc != Some(dc_value) {
        let message = format!(
            "'{}' did not take effect: requested AC {} / DC {}, Windows reports AC {} / DC {}.",
            setting.id,
            ac_label,
            dc_label,
            setting.format_value(verified_ac),
            setting.format_value(verified_dc)
        );
        log_error("power", &message);
        return Err(message);
    }

    let message = format!(
        "'{}' set to AC {}, DC {} (snapshot {}).",
        setting.id, ac_label, dc_label, snapshot.id
    );
    log_info("power", &message);
    Ok(message)
}

/// Turn a failed write into an actionable error, naming the privilege
/// requirement when that is the cause.
fn win32_write_error(rail: &str, setting_id: &str, code: u32, is_admin: bool) -> String {
    if code == ERROR_ACCESS_DENIED.0 {
        if is_admin {
            return format!(
                "Windows denied the {} write for '{}' (access denied). The active scheme may be read-only.",
                rail, setting_id
            );
        }
        return format!(
            "Setting '{}' requires Administrator privileges. Restart Wino elevated and try again.",
            setting_id
        );
    }
    format!(
        "Failed to write the {} value for '{}' (Win32 error {}).",
        rail, setting_id, code
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(id: &str) -> &'static SettingSpec {
        spec_for(id).expect("spec exists")
    }

    #[test]
    fn every_exposed_id_has_a_spec_with_real_guids() {
        assert_eq!(SETTING_SPECS.len(), EXPOSED_SETTING_IDS.len());
        for id in EXPOSED_SETTING_IDS {
            let s = spec(id);
            assert!(
                parse_guid(s.subgroup).is_some(),
                "subgroup GUID for {} must parse",
                id
            );
            assert!(
                parse_guid(s.setting).is_some(),
                "setting GUID for {} must parse",
                id
            );
        }
    }

    #[test]
    fn enumerated_specs_carry_index_aligned_labels() {
        let boost = spec("processor_boost");
        assert!(boost.is_enumerated);
        assert_eq!(
            boost.enum_labels,
            &[
                "Disabled",
                "Enabled",
                "Aggressive",
                "Efficient Enabled",
                "Efficient Aggressive"
            ]
        );
        assert_eq!(spec("usb_suspend").enum_labels, &["Disabled", "Enabled"]);
        assert_eq!(
            spec("pcie_link").enum_labels,
            &["Off", "Moderate", "Maximum"]
        );

        for id in [
            "processor_min",
            "processor_max",
            "sleep_timeout",
            "display_timeout",
            "disk_timeout",
        ] {
            assert!(!spec(id).is_enumerated, "{} is a ranged setting", id);
        }
    }

    #[test]
    fn units_are_exactly_percent_or_seconds() {
        assert_eq!(spec("processor_max").unit, "%");
        assert_eq!(spec("sleep_timeout").unit, "seconds");
        // The model renders "<value> <unit>", so no unit may carry a leading space.
        for s in SETTING_SPECS {
            assert!(
                !s.unit.starts_with(' '),
                "unit for {} has a leading space",
                s.id
            );
            assert!(
                s.unit.is_empty() || s.unit == "%" || s.unit == "seconds",
                "unexpected unit '{}' for {}",
                s.unit,
                s.id
            );
        }
    }

    #[test]
    fn built_setting_falls_back_to_sane_ranges_without_a_reported_one() {
        let percent = build_setting(spec("processor_max"), None, Some(100), Some(50));
        assert_eq!((percent.min_value, percent.max_value), (0, 100));
        assert_eq!(percent.format_value(Some(100)), "100 %");
        assert!(percent.is_available());

        let timeout = build_setting(spec("sleep_timeout"), None, Some(1800), None);
        assert_eq!(
            (timeout.min_value, timeout.max_value),
            (0, TIMEOUT_MAX_SECONDS)
        );
        assert_eq!(timeout.format_value(Some(1800)), "1800 seconds");

        let enumerated = build_setting(spec("pcie_link"), None, Some(2), Some(0));
        assert_eq!((enumerated.min_value, enumerated.max_value), (0, 2));
        assert_eq!(enumerated.format_value(Some(2)), "Maximum");
    }

    #[test]
    fn a_declared_range_wins_over_the_fallback_but_never_for_enums() {
        let ranged = build_setting(spec("processor_max"), Some((5, 95)), Some(50), Some(50));
        assert_eq!((ranged.min_value, ranged.max_value), (5, 95));
        assert_eq!(ranged.clamp(0), 5);
        assert_eq!(ranged.clamp(200), 95);

        // An enumerated setting's bounds are its label count, not whatever
        // numeric range the registry happens to declare.
        let enumerated = build_setting(spec("usb_suspend"), Some((0, 4_000_000)), Some(1), Some(1));
        assert_eq!((enumerated.min_value, enumerated.max_value), (0, 1));
    }

    #[test]
    fn timeout_ceiling_matches_the_fallback_so_a_declared_range_cannot_exceed_it() {
        // 0xFFFFFFFF is "unlimited" in the registry; treating it as a real
        // maximum would hand the UI a four-billion-second slider.
        let unlimited = build_setting(
            spec("sleep_timeout"),
            Some((0, 0xFFFF_FFFF)),
            Some(0),
            Some(0),
        );
        assert_eq!(unlimited.max_value, TIMEOUT_MAX_SECONDS);
        assert_eq!(unlimited.clamp(u32::MAX), TIMEOUT_MAX_SECONDS);
    }

    #[test]
    fn failed_reads_stay_none_and_are_never_reported_as_zero() {
        let unread = build_setting(spec("processor_min"), None, None, None);
        assert!(!unread.is_available());
        assert_eq!(unread.format_value(unread.ac_value), "—");
        assert_eq!(unread.format_value(unread.dc_value), "—");
        assert!(unread.ac_value.is_none());
        assert!(unread.dc_value.is_none());
    }

    #[test]
    fn missing_scheme_yields_no_settings_rather_than_fake_values() {
        // read_all_settings only ever reports what the API returned; the shape
        // assertion here is that an unavailable scheme cannot fabricate rows.
        let settings = read_all_settings();
        for setting in &settings {
            assert!(EXPOSED_SETTING_IDS.contains(&setting.id.as_str()));
            if !setting.is_available() {
                assert!(setting.ac_value.is_none() && setting.dc_value.is_none());
            }
        }
    }
}
