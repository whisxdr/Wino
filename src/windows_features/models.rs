//! Windows optional feature model.
//!
//! Feature state is read and changed through `dism.exe`, the documented Windows
//! interface for optional components. Wino never enables a feature on its own
//! initiative and never treats "unknown" as "disabled".

use crate::core::safety::RiskLevel;
use serde::{Deserialize, Serialize};

/// Reported state of an optional feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeatureState {
    Enabled,
    Disabled,
    /// The feature is in a transitional state (`EnablePending` / `DisablePending`).
    RequiresReboot,
    /// DISM did not report a state we can interpret.
    Unknown,
}

impl FeatureState {
    pub fn i18n_key(&self) -> &'static str {
        match self {
            FeatureState::Enabled => "features.state_enabled",
            FeatureState::Disabled => "features.state_disabled",
            FeatureState::RequiresReboot => "features.state_reboot",
            FeatureState::Unknown => "features.state_unknown",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            FeatureState::Enabled => "Enabled",
            FeatureState::Disabled => "Disabled",
            FeatureState::RequiresReboot => "Requires Reboot",
            FeatureState::Unknown => "Unknown",
        }
    }

    pub fn color_rgb(&self) -> (u8, u8, u8) {
        match self {
            FeatureState::Enabled => (34, 197, 94),
            FeatureState::Disabled => (100, 116, 139),
            FeatureState::RequiresReboot => (234, 179, 8),
            FeatureState::Unknown => (148, 163, 184),
        }
    }

    /// Parse a DISM state string. Unrecognized values map to [`FeatureState::Unknown`]
    /// rather than being guessed at.
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_lowercase().as_str() {
            "enabled" => FeatureState::Enabled,
            "disabled" => FeatureState::Disabled,
            "enablepending" | "disablepending" | "installpending" | "removepending" => {
                FeatureState::RequiresReboot
            }
            _ => FeatureState::Unknown,
        }
    }

    /// Whether Wino offers a toggle for this state.
    pub fn is_toggleable(&self) -> bool {
        matches!(self, FeatureState::Enabled | FeatureState::Disabled)
    }
}

/// One optional Windows feature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsFeature {
    /// Feature name as DISM reports it, e.g. `Microsoft-Hyper-V-All`.
    pub name: String,
    /// Human display name.
    pub display_name: String,
    pub state: FeatureState,
    /// Feature names this one depends on, as reported by DISM.
    pub dependencies: Vec<String>,
    /// Risk Wino assigns to changing this feature.
    pub risk: RiskLevel,
    /// What changing this feature affects. Shown before confirmation.
    pub impact: String,
    /// True when the feature is one Wino curates with a description; unknown
    /// features are still listed but without a curated warning.
    pub curated: bool,
}

impl WindowsFeature {
    /// Whether a toggle can be offered right now.
    pub fn can_toggle(&self) -> bool {
        self.state.is_toggleable() && !self.risk.is_hard_blocked()
    }

    /// The state a toggle would move to.
    pub fn target_state(&self) -> Option<FeatureState> {
        match self.state {
            FeatureState::Enabled => Some(FeatureState::Disabled),
            FeatureState::Disabled => Some(FeatureState::Enabled),
            _ => None,
        }
    }

    /// Dependencies that are currently disabled, which DISM would enable
    /// implicitly. Surfaced in the confirmation dialog so nothing changes
    /// silently.
    pub fn unmet_dependencies<'a>(&self, all: &'a [WindowsFeature]) -> Vec<&'a WindowsFeature> {
        self.dependencies
            .iter()
            .filter_map(|dep| {
                all.iter()
                    .find(|f| f.name.eq_ignore_ascii_case(dep) && f.state == FeatureState::Disabled)
            })
            .collect()
    }

    pub fn matches(&self, query_lower: &str) -> bool {
        if query_lower.is_empty() {
            return true;
        }
        self.name.to_lowercase().contains(query_lower)
            || self.display_name.to_lowercase().contains(query_lower)
    }
}

/// A curated feature entry: the description and risk Wino attaches to a
/// well-known optional component.
pub struct CuratedFeature {
    pub name: &'static str,
    pub display_name: &'static str,
    pub risk: RiskLevel,
    pub impact: &'static str,
}

/// Features Wino curates with an explicit warning. Anything not in this list is
/// still listed, but is treated as `Low` risk with a generic warning.
pub const CURATED_FEATURES: &[CuratedFeature] = &[
    CuratedFeature {
        name: "Microsoft-Hyper-V-All",
        display_name: "Hyper-V",
        risk: RiskLevel::Medium,
        impact: "Adds the Hyper-V hypervisor, management tools, and virtual switch services. Enabling changes how the CPU is virtualized and can affect other virtualization software and some anti-cheat systems.",
    },
    CuratedFeature {
        name: "VirtualMachinePlatform",
        display_name: "Virtual Machine Platform",
        risk: RiskLevel::Medium,
        impact: "Required by WSL 2 and Windows Sandbox. Enabling adds a hypervisor layer that other virtualization products may conflict with.",
    },
    CuratedFeature {
        name: "Containers-DisposableClientVM",
        display_name: "Windows Sandbox",
        risk: RiskLevel::Medium,
        impact: "Adds the Windows Sandbox feature and its dependencies. Sandbox sessions run a full Windows instance and need virtualization support.",
    },
    CuratedFeature {
        name: "OpenSSH.Client",
        display_name: "OpenSSH Client",
        risk: RiskLevel::Low,
        impact: "Installs the OpenSSH client tools (ssh, scp, sftp). No network listener is added by the client.",
    },
    CuratedFeature {
        name: "OpenSSH.Server",
        display_name: "OpenSSH Server",
        risk: RiskLevel::High,
        impact: "Installs an SSH server and registers a service that can accept inbound connections. This opens a network listener and should only be enabled deliberately.",
    },
    CuratedFeature {
        name: "SmbDirect",
        display_name: "SMB Direct",
        risk: RiskLevel::Low,
        impact: "Adds SMB over RDMA support. Only useful with RDMA-capable network hardware.",
    },
    CuratedFeature {
        name: "TelnetClient",
        display_name: "Telnet Client",
        risk: RiskLevel::Low,
        impact: "Installs the legacy telnet client. Telnet sends credentials in clear text; use SSH instead where possible.",
    },
    CuratedFeature {
        name: "IIS-WebServerRole",
        display_name: "Internet Information Services",
        risk: RiskLevel::High,
        impact: "Installs IIS and its web server role. Enabling starts web services and opens HTTP/HTTPS listeners on this machine.",
    },
    CuratedFeature {
        name: "IIS-WebServer",
        display_name: "IIS Web Server",
        risk: RiskLevel::High,
        impact: "Installs the IIS web server component and its HTTP listener.",
    },
    CuratedFeature {
        name: "NetFx3",
        display_name: ".NET Framework 3.5",
        risk: RiskLevel::Low,
        impact: "Adds .NET Framework 3.5 for applications that still require it. Needs a Windows Update source when the payload is not staged locally.",
    },
    CuratedFeature {
        name: "Windows-Defender-ApplicationGuard",
        display_name: "Windows Defender Application Guard",
        risk: RiskLevel::Medium,
        impact: "Adds hardware-isolated browsing for Microsoft Edge. Requires Hyper-V and virtualization support.",
    },
    CuratedFeature {
        name: "WorkFolders-Client",
        display_name: "Work Folders Client",
        risk: RiskLevel::Low,
        impact: "Adds the Work Folders client for syncing a corporate file share.",
    },
    CuratedFeature {
        name: "Printing-Foundation-Features",
        display_name: "Printing Foundation",
        risk: RiskLevel::Low,
        impact: "Core printing support. Disabling it removes printing capability from most applications.",
    },
    CuratedFeature {
        name: "MediaPlayback",
        display_name: "Media Playback",
        risk: RiskLevel::Low,
        impact: "Adds the media codecs used by Windows Media Player and other applications.",
    },
    CuratedFeature {
        name: "WindowsMediaPlayer",
        display_name: "Windows Media Player (legacy)",
        risk: RiskLevel::Low,
        impact: "Installs the legacy Windows Media Player. Media Playback is the modern replacement.",
    },
    CuratedFeature {
        name: "MicrosoftWindowsPowerShellV2",
        display_name: "Windows PowerShell 2.0 Engine",
        risk: RiskLevel::High,
        impact: "Restores the deprecated PowerShell 2.0 engine. It lacks modern logging and security features; enable only for legacy software that cannot run on PowerShell 5.1.",
    },
];

/// Look up the curated entry for a feature name (case-insensitive).
pub fn find_curated(name: &str) -> Option<&'static CuratedFeature> {
    CURATED_FEATURES
        .iter()
        .find(|c| c.name.eq_ignore_ascii_case(name))
}

/// Attach curation metadata to a scanned feature.
pub fn apply_curation(name: &str, display_name: &str) -> (String, RiskLevel, String, bool) {
    match find_curated(name) {
        Some(c) => (c.display_name.to_string(), c.risk, c.impact.to_string(), true),
        None => (
            if display_name.trim().is_empty() {
                name.to_string()
            } else {
                display_name.to_string()
            },
            RiskLevel::Low,
            "This optional component is not in Wino's curated list. Review the Windows documentation for its effect before changing it.".to_string(),
            false,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feature(name: &str, state: FeatureState, deps: &[&str]) -> WindowsFeature {
        WindowsFeature {
            name: name.to_string(),
            display_name: name.to_string(),
            state,
            dependencies: deps.iter().map(|d| d.to_string()).collect(),
            risk: RiskLevel::Low,
            impact: String::new(),
            curated: false,
        }
    }

    #[test]
    fn state_parsing_never_guesses() {
        assert_eq!(FeatureState::parse("Enabled"), FeatureState::Enabled);
        assert_eq!(FeatureState::parse(" disabled "), FeatureState::Disabled);
        assert_eq!(
            FeatureState::parse("EnablePending"),
            FeatureState::RequiresReboot
        );
        assert_eq!(
            FeatureState::parse("RemovePending"),
            FeatureState::RequiresReboot
        );
        assert_eq!(FeatureState::parse("SomethingElse"), FeatureState::Unknown);
        assert_eq!(FeatureState::parse(""), FeatureState::Unknown);
    }

    #[test]
    fn unknown_and_pending_states_are_not_toggleable() {
        assert!(FeatureState::Enabled.is_toggleable());
        assert!(FeatureState::Disabled.is_toggleable());
        assert!(!FeatureState::Unknown.is_toggleable());
        assert!(!FeatureState::RequiresReboot.is_toggleable());

        assert_eq!(
            feature("F", FeatureState::Enabled, &[]).target_state(),
            Some(FeatureState::Disabled)
        );
        assert_eq!(
            feature("F", FeatureState::Disabled, &[]).target_state(),
            Some(FeatureState::Enabled)
        );
        assert_eq!(
            feature("F", FeatureState::Unknown, &[]).target_state(),
            None
        );
    }

    #[test]
    fn unmet_dependencies_report_only_disabled_ones() {
        let all = vec![
            feature("VirtualMachinePlatform", FeatureState::Disabled, &[]),
            feature("Hyper-V", FeatureState::Enabled, &[]),
        ];
        let target = feature(
            "WindowsSandbox",
            FeatureState::Disabled,
            &["VirtualMachinePlatform", "Hyper-V"],
        );

        let unmet = target.unmet_dependencies(&all);
        assert_eq!(unmet.len(), 1);
        assert_eq!(unmet[0].name, "VirtualMachinePlatform");
    }

    #[test]
    fn curation_covers_listed_features_and_falls_back_safely() {
        let (name, risk, impact, curated) = apply_curation("Microsoft-Hyper-V-All", "Hyper-V");
        assert_eq!(name, "Hyper-V");
        assert_eq!(risk, RiskLevel::Medium);
        assert!(impact.contains("hypervisor"));
        assert!(curated);

        let (name, risk, impact, curated) = apply_curation("Some-Unknown-Feature", "Friendly");
        assert_eq!(name, "Friendly");
        assert_eq!(risk, RiskLevel::Low);
        assert!(!impact.is_empty());
        assert!(!curated);

        // Case-insensitive lookup, and a missing display name falls back to the raw name.
        assert!(find_curated("openssh.server").is_some());
        let (name, _, _, _) = apply_curation("OpenSSH.Server", "   ");
        assert_eq!(name, "OpenSSH Server");
    }

    #[test]
    fn high_risk_features_are_declared_for_listeners() {
        let ssh_server = find_curated("OpenSSH.Server").expect("OpenSSH.Server curated");
        assert_eq!(ssh_server.risk, RiskLevel::High);
        let iis = find_curated("IIS-WebServerRole").expect("IIS curated");
        assert_eq!(iis.risk, RiskLevel::High);
    }
}
