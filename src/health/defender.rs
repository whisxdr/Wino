use serde::{Deserialize, Serialize};
use windows::core::PCWSTR;
use windows::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_VALUE_TYPE,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityHealth {
    pub defender_enabled: bool,
    pub firewall_enabled: bool,
    pub real_time_protection: bool,
}

fn query_dword_value(subkey: &str, value_name: &str) -> Option<u32> {
    let subkey_wide: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
    let value_wide: Vec<u16> = value_name.encode_utf16().chain(std::iter::once(0)).collect();

    let mut key = HKEY::default();
    unsafe {
        if RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(subkey_wide.as_ptr()),
            0,
            KEY_READ,
            &mut key,
        ).is_ok() {
            let mut val_type = REG_VALUE_TYPE::default();
            let mut data = 0u32;
            let mut data_size = std::mem::size_of::<u32>() as u32;

            let res = RegQueryValueExW(
                key,
                PCWSTR(value_wide.as_ptr()),
                None,
                Some(&mut val_type),
                Some(&mut data as *mut _ as *mut _),
                Some(&mut data_size),
            );

            let _ = RegCloseKey(key);

            if res.is_ok() {
                return Some(data);
            }
        }
    }
    None
}

pub fn check_security_health() -> SecurityHealth {
    // Check Defender registry values natively without spawning PowerShell
    let disable_av = query_dword_value(
        "SOFTWARE\\Microsoft\\Windows Defender",
        "DisableAntiVirus",
    ).unwrap_or(0);

    let disable_rt = query_dword_value(
        "SOFTWARE\\Microsoft\\Windows Defender\\Real-Time Protection",
        "DisableRealtimeMonitoring",
    ).unwrap_or(0);

    let defender_enabled = disable_av == 0;
    let real_time_protection = disable_av == 0 && disable_rt == 0;

    // Check Windows Firewall status natively
    let firewall_enabled = query_dword_value(
        "SYSTEM\\CurrentControlSet\\Services\\SharedAccess\\Parameters\\FirewallPolicy\\StandardProfile",
        "EnableFirewall",
    ).map(|v| v != 0).unwrap_or(true);

    SecurityHealth {
        defender_enabled,
        firewall_enabled,
        real_time_protection,
    }
}
