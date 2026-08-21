use serde::{Deserialize, Serialize};
use windows::core::PCWSTR;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ};
use windows::Win32::System::SystemInformation::{GetNativeSystemInfo, SYSTEM_INFO};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os_name: String,
    pub display_version: String,
    pub build_number: String,
    pub architecture: String,
    pub processor_count: u32,
    pub total_ram_bytes: u64,
    pub is_windows_11: bool,
    pub is_admin: bool,
}

impl SystemInfo {
    pub fn detect() -> Self {
        let (os_name, display_version, build_number) = read_windows_version_registry();
        let build_num_int: u32 = build_number.parse().unwrap_or(0);
        let is_windows_11 = build_num_int >= 22000;

        let mut sys_info = SYSTEM_INFO::default();
        let architecture = unsafe {
            GetNativeSystemInfo(&mut sys_info);
            match sys_info.Anonymous.Anonymous.wProcessorArchitecture.0 {
                9 => "x86_64 (64-bit)".to_string(),
                5 => "ARM".to_string(),
                12 => "ARM64".to_string(),
                0 => "x86 (32-bit)".to_string(),
                _ => "Unknown".to_string(),
            }
        };

        let processor_count = sys_info.dwNumberOfProcessors;

        let total_ram_bytes = crate::monitoring::ram::get_total_ram_bytes();
        let is_admin = crate::core::permissions::is_admin();

        Self {
            os_name,
            display_version,
            build_number,
            architecture,
            processor_count,
            total_ram_bytes,
            is_windows_11,
            is_admin,
        }
    }
}

fn read_windows_version_registry() -> (String, String, String) {
    let subkey_wide: Vec<u16> = "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\0".encode_utf16().collect();
    let mut hkey = HKEY::default();

    unsafe {
        let open_res = RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(subkey_wide.as_ptr()),
            0,
            KEY_READ,
            &mut hkey,
        );

        if open_res != ERROR_SUCCESS {
            return ("Windows".to_string(), "".to_string(), "".to_string());
        }

        let product_name = read_reg_string(hkey, "ProductName").unwrap_or_else(|| "Windows".to_string());
        let display_version = read_reg_string(hkey, "DisplayVersion").unwrap_or_else(|| "".to_string());
        let current_build = read_reg_string(hkey, "CurrentBuildNumber").unwrap_or_else(|| "".to_string());

        let _ = RegCloseKey(hkey);

        (product_name, display_version, current_build)
    }
}

pub fn read_reg_string(hkey: HKEY, value_name: &str) -> Option<String> {
    let val_name_wide: Vec<u16> = value_name.encode_utf16().chain(std::iter::once(0)).collect();
    let mut val_type = REG_SZ;
    let mut data_size = 0u32;

    unsafe {
        let size_res = RegQueryValueExW(
            hkey,
            PCWSTR(val_name_wide.as_ptr()),
            None,
            Some(&mut val_type),
            None,
            Some(&mut data_size),
        );

        if size_res != ERROR_SUCCESS || data_size == 0 {
            return None;
        }

        let mut buffer: Vec<u8> = vec![0; data_size as usize];
        let query_res = RegQueryValueExW(
            hkey,
            PCWSTR(val_name_wide.as_ptr()),
            None,
            Some(&mut val_type),
            Some(buffer.as_mut_ptr()),
            Some(&mut data_size),
        );

        if query_res != ERROR_SUCCESS {
            return None;
        }

        // Convert UTF-16 bytes to String
        let u16_slice: &[u16] = std::slice::from_raw_parts(
            buffer.as_ptr() as *const u16,
            (data_size as usize) / 2,
        );

        let s = String::from_utf16_lossy(u16_slice);
        Some(s.trim_matches('\0').trim().to_string())
    }
}
