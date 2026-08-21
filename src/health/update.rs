use serde::{Deserialize, Serialize};
use windows::core::PCWSTR;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{RegCloseKey, RegOpenKeyExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateHealth {
    pub service_active: bool,
    pub pending_reboot: bool,
}

pub fn check_update_health() -> UpdateHealth {
    let pending_reboot = check_reboot_required();
    UpdateHealth {
        service_active: true,
        pending_reboot,
    }
}

fn check_reboot_required() -> bool {
    let key_path: Vec<u16> = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\WindowsUpdate\\Auto Update\\RebootRequired\0".encode_utf16().collect();
    let mut hkey = HKEY::default();

    unsafe {
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(key_path.as_ptr()), 0, KEY_READ, &mut hkey) == ERROR_SUCCESS {
            let _ = RegCloseKey(hkey);
            true
        } else {
            false
        }
    }
}
