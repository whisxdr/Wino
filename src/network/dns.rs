use serde::{Deserialize, Serialize};
use windows::Win32::System::Registry::HKEY_LOCAL_MACHINE;

use crate::core::regutil;

const INTERFACES_KEY: &str = "SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters\\Interfaces";
const NETWORK_CONNECTIONS: &str = "SYSTEM\\CurrentControlSet\\Control\\Network";

/// A curated high-speed, privacy-friendly DNS preset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DnsPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub primary: Option<&'static str>,
    pub secondary: Option<&'static str>,
}

impl DnsPreset {
    /// Comma-joined server list as stored in the `NameServer` registry value.
    pub fn name_server_value(&self) -> String {
        let mut servers = Vec::new();
        if let Some(p) = self.primary {
            servers.push(p);
        }
        if let Some(s) = self.secondary {
            servers.push(s);
        }
        servers.join(",")
    }
}

pub const PRESET_AUTOMATIC: DnsPreset = DnsPreset {
    id: "auto",
    name: "Automatic (DHCP)",
    primary: None,
    secondary: None,
};
pub const PRESET_CLOUDFLARE: DnsPreset = DnsPreset {
    id: "cloudflare",
    name: "Cloudflare",
    primary: Some("1.1.1.1"),
    secondary: Some("1.0.0.1"),
};
pub const PRESET_GOOGLE: DnsPreset = DnsPreset {
    id: "google",
    name: "Google",
    primary: Some("8.8.8.8"),
    secondary: Some("8.8.4.4"),
};
pub const PRESET_QUAD9: DnsPreset = DnsPreset {
    id: "quad9",
    name: "Quad9",
    primary: Some("9.9.9.9"),
    secondary: Some("149.112.112.112"),
};

pub const ALL_PRESETS: &[DnsPreset] = &[
    PRESET_AUTOMATIC,
    PRESET_CLOUDFLARE,
    PRESET_GOOGLE,
    PRESET_QUAD9,
];

pub fn find_preset(id: &str) -> Option<&'static DnsPreset> {
    ALL_PRESETS.iter().find(|p| p.id == id)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterInfo {
    /// Interface GUID, e.g. "{4D36E972-E325-11CE-BFC1-08002BE10318}"
    pub guid: String,
    pub friendly_name: String,
    pub current_name_server: String,
}

/// Enumerate TCP/IP interfaces with their friendly connection names.
pub fn list_adapters() -> Vec<AdapterInfo> {
    let mut adapters = Vec::new();

    for guid in regutil::enum_subkeys(HKEY_LOCAL_MACHINE, INTERFACES_KEY) {
        let interface_path = format!("{}\\{}", INTERFACES_KEY, guid);
        // Skip WAN Miniports / loopback pseudo-interfaces without an IP config.
        if regutil::read_string(HKEY_LOCAL_MACHINE, &interface_path, "NameServer").is_none()
            && regutil::read_string(HKEY_LOCAL_MACHINE, &interface_path, "EnableDHCP").is_none()
        {
            continue;
        }

        let conn_path = format!("{}\\{}\\Connection", NETWORK_CONNECTIONS, guid);
        let friendly = regutil::read_string(HKEY_LOCAL_MACHINE, &conn_path, "Name")
            .unwrap_or_else(|| guid.clone());

        adapters.push(AdapterInfo {
            guid,
            friendly_name: friendly,
            current_name_server: regutil::read_string(
                HKEY_LOCAL_MACHINE,
                &interface_path,
                "NameServer",
            )
            .unwrap_or_default(),
        });
    }

    adapters
}

/// Apply a DNS preset to one adapter (writes the NameServer override natively,
/// no subprocess involved). Empty preset id "auto" clears the static override
/// so DHCP-provided DNS takes over again.
pub fn set_adapter_dns(
    adapter: &AdapterInfo,
    preset: &DnsPreset,
    dry_run: bool,
) -> Result<(), String> {
    let interface_path = format!("{}\\{}", INTERFACES_KEY, adapter.guid);

    if dry_run {
        return Ok(());
    }

    // Backup previous value into a snapshot for one-click rollback.
    let prev = regutil::read_string(HKEY_LOCAL_MACHINE, &interface_path, "NameServer");
    crate::restore::snapshots::create_snapshot_with_string_entry(
        &format!("DNS change '{}' on {}", preset.name, adapter.friendly_name),
        &crate::restore::snapshots::StringBackupEntry {
            hive: "HKLM".to_string(),
            path: interface_path.clone(),
            value_name: "NameServer".to_string(),
            previous_value: prev,
        },
    );

    match (preset.primary, preset.secondary) {
        (None, _) => regutil::delete_value(HKEY_LOCAL_MACHINE, &interface_path, "NameServer"),
        (Some(primary), secondary) => {
            let value = match secondary {
                Some(sec) => format!("{},{}", primary, sec),
                None => primary.to_string(),
            };
            regutil::write_string(HKEY_LOCAL_MACHINE, &interface_path, "NameServer", &value)
        }
    }
}
