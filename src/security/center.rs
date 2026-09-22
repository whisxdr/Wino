//! Security Center.
//!
//! Reports protection state only. Wino deliberately exposes no mechanism for
//! turning a protection off as an "optimization" — a disabled protection is
//! always reported as a warning, never as an opportunity.

use serde::{Deserialize, Serialize};

/// State of one protection component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtectionState {
    On,
    Off,
    /// Present but not in a state we can call protected.
    Warning,
    /// Could not be queried. Never rendered as "On".
    Unknown,
}

impl ProtectionState {
    pub fn i18n_key(&self) -> &'static str {
        match self {
            ProtectionState::On => "security.state_on",
            ProtectionState::Off => "security.state_off",
            ProtectionState::Warning => "security.state_warning",
            ProtectionState::Unknown => "security.state_unknown",
        }
    }

    pub fn color_rgb(&self) -> (u8, u8, u8) {
        match self {
            ProtectionState::On => (34, 197, 94),
            ProtectionState::Off => (239, 68, 68),
            ProtectionState::Warning => (234, 179, 8),
            ProtectionState::Unknown => (148, 163, 184),
        }
    }

    /// Whether this state should raise a warning banner.
    pub fn needs_attention(&self) -> bool {
        matches!(self, ProtectionState::Off | ProtectionState::Warning)
    }
}

/// One protection component with its observed state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectionItem {
    /// i18n key for the component name.
    pub name_key: String,
    pub state: ProtectionState,
    /// What was actually observed, or why the query failed.
    pub detail: String,
    /// Extra context, e.g. the TPM version.
    pub extra: String,
}

impl ProtectionItem {
    pub fn new(name_key: &str, state: ProtectionState, detail: &str) -> Self {
        Self {
            name_key: name_key.to_string(),
            state,
            detail: detail.to_string(),
            extra: String::new(),
        }
    }

    pub fn with_extra(mut self, extra: &str) -> Self {
        self.extra = extra.to_string();
        self
    }
}

/// Full security report.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecurityReport {
    pub items: Vec<ProtectionItem>,
}

impl SecurityReport {
    /// Components that need attention.
    pub fn warnings(&self) -> Vec<&ProtectionItem> {
        self.items
            .iter()
            .filter(|i| i.state.needs_attention())
            .collect()
    }

    /// Components that could not be queried.
    pub fn unknowns(&self) -> Vec<&ProtectionItem> {
        self.items
            .iter()
            .filter(|i| i.state == ProtectionState::Unknown)
            .collect()
    }

    /// Count of protections observed on.
    pub fn on_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.state == ProtectionState::On)
            .count()
    }

    /// True when every component was queried and none needs attention.
    pub fn is_fully_protected(&self) -> bool {
        !self.items.is_empty() && self.items.iter().all(|i| i.state == ProtectionState::On)
    }
}

/// Classify the Defender real-time protection registry values.
///
/// Absent values mean "not disabled", which is the Windows default, but a
/// *disabled* value is reported explicitly rather than being treated as a
/// generic failure.
pub fn classify_defender(disable_av: Option<u32>, disable_rt: Option<u32>) -> ProtectionState {
    let av_off = disable_av == Some(1);
    let rt_off = disable_rt == Some(1);
    if av_off {
        ProtectionState::Off
    } else if rt_off {
        ProtectionState::Warning
    } else {
        ProtectionState::On
    }
}

/// Classify a firewall registry value.
pub fn classify_firewall(enable_firewall: Option<u32>) -> ProtectionState {
    match enable_firewall {
        Some(0) => ProtectionState::Off,
        Some(_) => ProtectionState::On,
        // No value at all: the profile key is absent, so the state is unknown
        // rather than assumed on.
        None => ProtectionState::Unknown,
    }
}

/// Classify the Secure Boot state from `UEFISecureBootEnabled`.
pub fn classify_secure_boot(value: Option<u32>) -> ProtectionState {
    match value {
        Some(1) => ProtectionState::On,
        Some(0) => ProtectionState::Off,
        // A value outside 0/1 is not a state Windows defines, so it is reported
        // as unknown rather than folded into on or off.
        Some(_) => ProtectionState::Unknown,
        // Absent usually means legacy BIOS boot, where Secure Boot does not
        // exist. That is a real, reportable condition rather than an error.
        None => ProtectionState::Unknown,
    }
}

/// Classify the UAC enable level (`EnableLUA`).
pub fn classify_uac(enable_lua: Option<u32>) -> ProtectionState {
    match enable_lua {
        Some(0) => ProtectionState::Off,
        Some(_) => ProtectionState::On,
        None => ProtectionState::Unknown,
    }
}

/// Classify the SmartScreen app-reputation value.
///
/// SmartScreen can be legitimately "warn" or "off" by user choice, so a
/// non-on value is a warning rather than a hard failure.
pub fn classify_smartscreen(enable_smartscreen: Option<u32>) -> ProtectionState {
    match enable_smartscreen {
        Some(1) => ProtectionState::On,
        Some(0) => ProtectionState::Warning,
        Some(_) => ProtectionState::On,
        None => ProtectionState::Unknown,
    }
}

/// Classify TPM presence and readiness from the `Security` WMI-free registry
/// surface: `SpecVersion` present means a TPM is present.
pub fn classify_tpm(spec_version: Option<&str>) -> ProtectionState {
    match spec_version {
        Some(v) if !v.trim().is_empty() => ProtectionState::On,
        Some(_) => ProtectionState::Unknown,
        None => ProtectionState::Off,
    }
}

/// Classify Windows Update service state for the security view.
pub fn classify_update_service(state: Option<&str>) -> ProtectionState {
    match state {
        Some("Running") => ProtectionState::On,
        Some("Stopped") => ProtectionState::Warning,
        Some(_) => ProtectionState::Unknown,
        None => ProtectionState::Unknown,
    }
}

/// Classify a set of critical service states into one verdict.
///
/// Missing services are reported as `Warning` (something removed a component
/// Windows expects to exist) while stopped services are `Off`.
pub fn classify_critical_services(
    states: &[(String, Option<String>)],
) -> (ProtectionState, String) {
    if states.is_empty() {
        return (
            ProtectionState::Unknown,
            "No services were queried.".to_string(),
        );
    }

    let mut missing = Vec::new();
    let mut stopped = Vec::new();
    for (name, state) in states {
        match state.as_deref() {
            Some("Running") => {}
            Some("Stopped") => stopped.push(name.clone()),
            Some(_) => stopped.push(name.clone()),
            None => missing.push(name.clone()),
        }
    }

    if !stopped.is_empty() {
        (
            ProtectionState::Off,
            format!("Not running: {}.", stopped.join(", ")),
        )
    } else if !missing.is_empty() {
        (
            ProtectionState::Warning,
            format!("Not present on this system: {}.", missing.join(", ")),
        )
    } else {
        (
            ProtectionState::On,
            format!("All {} services are running.", states.len()),
        )
    }
}

/// Critical security services whose state Wino reports.
pub const CRITICAL_SECURITY_SERVICES: &[&str] = &[
    "WinDefend",
    "MpsSvc",
    "BFE",
    "wuauserv",
    "SecurityHealthService",
];

/// Read a `HKLM` DWORD value, `None` when absent.
fn read_hklm_dword(path: &str, name: &str) -> Option<u32> {
    crate::core::executor::SystemExecutor::read_registry_dword("HKLM", path, name)
}

/// Read a `HKLM` string value, `None` when absent.
fn read_hklm_string(path: &str, name: &str) -> Option<String> {
    use windows::Win32::System::Registry::HKEY_LOCAL_MACHINE;
    crate::core::regutil::read_string(HKEY_LOCAL_MACHINE, path, name)
}

/// Scan every protection component.
pub fn scan_security_center() -> SecurityReport {
    let mut items = Vec::new();

    // ---- Defender ----
    let disable_av = read_hklm_dword("SOFTWARE\\Microsoft\\Windows Defender", "DisableAntiVirus");
    let disable_rt = read_hklm_dword(
        "SOFTWARE\\Microsoft\\Windows Defender\\Real-Time Protection",
        "DisableRealtimeMonitoring",
    );
    let defender_state = classify_defender(disable_av, disable_rt);
    let defender_detail = match defender_state {
        ProtectionState::On => "No disable flags are set in the Defender policy keys.",
        ProtectionState::Warning => "Real-time monitoring is disabled by policy.",
        ProtectionState::Off => "Antivirus is disabled by policy.",
        ProtectionState::Unknown => "Defender policy keys could not be read.",
    };
    items.push(ProtectionItem::new(
        "security.defender",
        defender_state,
        defender_detail,
    ));

    // ---- Firewall ----
    let fw = read_hklm_dword(
        "SYSTEM\\CurrentControlSet\\Services\\SharedAccess\\Parameters\\FirewallPolicy\\StandardProfile",
        "EnableFirewall",
    );
    let fw_state = classify_firewall(fw);
    let fw_detail = match fw_state {
        ProtectionState::On => "The standard profile has the firewall enabled.",
        ProtectionState::Off => "The standard profile has the firewall disabled.",
        ProtectionState::Warning => "The firewall is enabled but not fully configured.",
        ProtectionState::Unknown => "The firewall policy key could not be read.",
    };
    items.push(ProtectionItem::new(
        "security.firewall",
        fw_state,
        fw_detail,
    ));

    // ---- Secure Boot ----
    let sb = read_hklm_dword(
        "SYSTEM\\CurrentControlSet\\Control\\SecureBoot\\State",
        "UEFISecureBootEnabled",
    );
    let sb_state = classify_secure_boot(sb);
    let sb_detail = match sb_state {
        ProtectionState::On => "Firmware reports Secure Boot enabled.",
        ProtectionState::Off => "Firmware reports Secure Boot disabled.",
        ProtectionState::Warning => "Secure Boot is in a reduced state.",
        ProtectionState::Unknown => {
            "Secure Boot state is not exposed. This is expected on a legacy BIOS boot."
        }
    };
    items.push(ProtectionItem::new(
        "security.secure_boot",
        sb_state,
        sb_detail,
    ));

    // ---- TPM ----
    let spec = read_hklm_string(
        "SYSTEM\\CurrentControlSet\\Services\\TPM\\WMI\\Admin",
        "SpecVersion",
    );
    let tpm_state = classify_tpm(spec.as_deref());
    let tpm_item = ProtectionItem::new(
        "security.tpm",
        tpm_state,
        match tpm_state {
            ProtectionState::On => "A TPM is present and reports a specification version.",
            ProtectionState::Off => "No TPM was detected on this system.",
            _ => "TPM state could not be determined.",
        },
    )
    .with_extra(spec.as_deref().unwrap_or(""));
    items.push(tpm_item);

    // ---- UAC ----
    let lua = read_hklm_dword(
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\System",
        "EnableLUA",
    );
    let uac_state = classify_uac(lua);
    let uac_detail = match uac_state {
        ProtectionState::On => "User Account Control is enabled.",
        ProtectionState::Off => "User Account Control is disabled. Applications run with the full token of the signed-in user.",
        ProtectionState::Warning => "User Account Control is in a reduced state.",
        ProtectionState::Unknown => "The UAC policy value could not be read.",
    };
    items.push(ProtectionItem::new("security.uac", uac_state, uac_detail));

    // ---- SmartScreen ----
    let ss = read_hklm_dword(
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer",
        "SmartScreenEnabled",
    );
    let ss_state = classify_smartscreen(ss);
    let ss_detail = match ss_state {
        ProtectionState::On => "SmartScreen is enabled for applications.",
        ProtectionState::Warning => "SmartScreen is disabled or set to warn only.",
        ProtectionState::Off => "SmartScreen is off.",
        ProtectionState::Unknown => "The SmartScreen value could not be read.",
    };
    items.push(ProtectionItem::new(
        "security.smartscreen",
        ss_state,
        ss_detail,
    ));

    // ---- Windows Update ----
    let wu_state = crate::services::scanner::service_state_query("wuauserv");
    let update_state = classify_update_service(wu_state.as_deref());
    let update_detail = match &wu_state {
        Some(s) => format!("Windows Update service state: {}.", s),
        None => "The Windows Update service state could not be queried.".to_string(),
    };
    items.push(ProtectionItem::new(
        "security.windows_update",
        update_state,
        &update_detail,
    ));

    // ---- Critical security services ----
    let states: Vec<(String, Option<String>)> = CRITICAL_SECURITY_SERVICES
        .iter()
        .map(|name| {
            (
                (*name).to_string(),
                crate::services::scanner::service_state_query(name),
            )
        })
        .collect();
    let (svc_state, svc_detail) = classify_critical_services(&states);
    items.push(ProtectionItem::new(
        "security.critical_services",
        svc_state,
        &svc_detail,
    ));

    SecurityReport { items }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defender_classification_distinguishes_off_from_warning() {
        assert_eq!(classify_defender(None, None), ProtectionState::On);
        assert_eq!(classify_defender(Some(0), Some(0)), ProtectionState::On);
        assert_eq!(classify_defender(Some(1), Some(0)), ProtectionState::Off);
        assert_eq!(
            classify_defender(Some(0), Some(1)),
            ProtectionState::Warning
        );
    }

    #[test]
    fn firewall_absent_value_is_unknown_not_assumed_on() {
        assert_eq!(classify_firewall(Some(1)), ProtectionState::On);
        assert_eq!(classify_firewall(Some(0)), ProtectionState::Off);
        assert_eq!(classify_firewall(None), ProtectionState::Unknown);
    }

    #[test]
    fn secure_boot_absent_value_is_unknown() {
        assert_eq!(classify_secure_boot(Some(1)), ProtectionState::On);
        assert_eq!(classify_secure_boot(Some(0)), ProtectionState::Off);
        assert_eq!(classify_secure_boot(None), ProtectionState::Unknown);
    }

    #[test]
    fn uac_and_smartscreen_classification() {
        assert_eq!(classify_uac(Some(1)), ProtectionState::On);
        assert_eq!(classify_uac(Some(0)), ProtectionState::Off);
        assert_eq!(classify_uac(None), ProtectionState::Unknown);

        assert_eq!(classify_smartscreen(Some(1)), ProtectionState::On);
        assert_eq!(classify_smartscreen(Some(0)), ProtectionState::Warning);
        assert_eq!(classify_smartscreen(None), ProtectionState::Unknown);
    }

    #[test]
    fn tpm_classification_requires_a_spec_version() {
        assert_eq!(classify_tpm(Some("2.0")), ProtectionState::On);
        assert_eq!(classify_tpm(Some("  ")), ProtectionState::Unknown);
        assert_eq!(classify_tpm(None), ProtectionState::Off);
    }

    #[test]
    fn update_service_classification_marks_stopped_as_warning() {
        assert_eq!(
            classify_update_service(Some("Running")),
            ProtectionState::On
        );
        assert_eq!(
            classify_update_service(Some("Stopped")),
            ProtectionState::Warning
        );
        assert_eq!(classify_update_service(None), ProtectionState::Unknown);
    }

    #[test]
    fn critical_services_prefer_stopped_over_missing() {
        let states = vec![
            ("WinDefend".to_string(), Some("Running".to_string())),
            ("MpsSvc".to_string(), Some("Stopped".to_string())),
            ("Ghost".to_string(), None),
        ];
        let (state, detail) = classify_critical_services(&states);
        assert_eq!(state, ProtectionState::Off);
        assert!(detail.contains("MpsSvc"));
        assert!(!detail.contains("Ghost"));
    }

    #[test]
    fn critical_services_all_running_is_on_and_missing_only_is_warning() {
        let running = vec![
            ("WinDefend".to_string(), Some("Running".to_string())),
            ("BFE".to_string(), Some("Running".to_string())),
        ];
        let (state, detail) = classify_critical_services(&running);
        assert_eq!(state, ProtectionState::On);
        assert!(detail.contains("2 services"));

        let missing = vec![("Ghost".to_string(), None)];
        let (state, detail) = classify_critical_services(&missing);
        assert_eq!(state, ProtectionState::Warning);
        assert!(detail.contains("Ghost"));

        let (empty_state, _) = classify_critical_services(&[]);
        assert_eq!(empty_state, ProtectionState::Unknown);
    }

    #[test]
    fn report_helpers_count_and_rank_correctly() {
        let report = SecurityReport {
            items: vec![
                ProtectionItem::new("a", ProtectionState::On, ""),
                ProtectionItem::new("b", ProtectionState::Off, ""),
                ProtectionItem::new("c", ProtectionState::Unknown, ""),
            ],
        };
        assert_eq!(report.on_count(), 1);
        assert_eq!(report.warnings().len(), 1);
        assert_eq!(report.unknowns().len(), 1);
        assert!(!report.is_fully_protected());

        let all_on = SecurityReport {
            items: vec![ProtectionItem::new("a", ProtectionState::On, "")],
        };
        assert!(all_on.is_fully_protected());
        assert!(!SecurityReport::default().is_fully_protected());
    }

    #[test]
    fn attention_states_are_off_and_warning_only() {
        assert!(ProtectionState::Off.needs_attention());
        assert!(ProtectionState::Warning.needs_attention());
        assert!(!ProtectionState::On.needs_attention());
        assert!(!ProtectionState::Unknown.needs_attention());
    }
}
