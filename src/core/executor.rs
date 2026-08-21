use windows::core::PCWSTR;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE, REG_DWORD, REG_OPTION_NON_VOLATILE,
};
use crate::core::logger::{log_error, log_info};

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub success: bool,
    pub dry_run: bool,
    pub action: String,
    pub details: String,
}

pub struct SystemExecutor;

impl SystemExecutor {
    pub fn get_hive(hive_str: &str) -> Option<HKEY> {
        match hive_str.to_uppercase().as_str() {
            "HKCU" | "HKEY_CURRENT_USER" => Some(HKEY_CURRENT_USER),
            "HKLM" | "HKEY_LOCAL_MACHINE" => Some(HKEY_LOCAL_MACHINE),
            _ => None,
        }
    }

    pub fn set_registry_dword(
        hive_str: &str,
        path: &str,
        name: &str,
        value: u32,
        dry_run: bool,
    ) -> ExecutionResult {
        let action = format!("Set Registry DWORD: [{}\\{}] {} = {}", hive_str, path, name, value);

        if dry_run {
            log_info("executor", &format!("[DRY-RUN] {}", action));
            return ExecutionResult {
                success: true,
                dry_run: true,
                action,
                details: "Would write DWORD value to registry.".to_string(),
            };
        }

        let Some(hive) = Self::get_hive(hive_str) else {
            return ExecutionResult {
                success: false,
                dry_run: false,
                action,
                details: format!("Invalid registry hive: {}", hive_str),
            };
        };

        let path_wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let name_wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        let mut hkey = HKEY::default();

        unsafe {
            let create_res = RegCreateKeyExW(
                hive,
                PCWSTR(path_wide.as_ptr()),
                0,
                None,
                REG_OPTION_NON_VOLATILE,
                KEY_WRITE | KEY_READ,
                None,
                &mut hkey,
                None,
            );

            if create_res != ERROR_SUCCESS {
                log_error("executor", &format!("Failed to open/create registry key: {}\\{}", hive_str, path));
                return ExecutionResult {
                    success: false,
                    dry_run: false,
                    action,
                    details: format!("Failed to open registry key (Win32 Error: {:?})", create_res),
                };
            }

            let val_bytes = value.to_ne_bytes();
            let set_res = RegSetValueExW(
                hkey,
                PCWSTR(name_wide.as_ptr()),
                0,
                REG_DWORD,
                Some(&val_bytes),
            );

            let _ = RegCloseKey(hkey);

            if set_res == ERROR_SUCCESS {
                log_info("executor", &format!("Applied: {}", action));
                ExecutionResult {
                    success: true,
                    dry_run: false,
                    action,
                    details: "Registry value applied successfully.".to_string(),
                }
            } else {
                log_error("executor", &format!("Failed to set registry value: {}", name));
                ExecutionResult {
                    success: false,
                    dry_run: false,
                    action,
                    details: format!("Failed to write registry value (Win32 Error: {:?})", set_res),
                }
            }
        }
    }

    pub fn read_registry_dword(hive_str: &str, path: &str, name: &str) -> Option<u32> {
        let hive = Self::get_hive(hive_str)?;
        let path_wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let name_wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        let mut hkey = HKEY::default();

        unsafe {
            let open_res = RegOpenKeyExW(
                hive,
                PCWSTR(path_wide.as_ptr()),
                0,
                KEY_READ,
                &mut hkey,
            );

            if open_res != ERROR_SUCCESS {
                return None;
            }

            let mut val_type = REG_DWORD;
            let mut val_data = 0u32;
            let mut data_size = std::mem::size_of::<u32>() as u32;

            let query_res = RegQueryValueExW(
                hkey,
                PCWSTR(name_wide.as_ptr()),
                None,
                Some(&mut val_type),
                Some(&mut val_data as *mut _ as *mut _),
                Some(&mut data_size),
            );

            let _ = RegCloseKey(hkey);

            if query_res == ERROR_SUCCESS && val_type == REG_DWORD {
                Some(val_data)
            } else {
                None
            }
        }
    }

    pub fn delete_registry_value(hive_str: &str, path: &str, name: &str, dry_run: bool) -> ExecutionResult {
        let action = format!("Delete Registry Value: [{}\\{}] {}", hive_str, path, name);

        if dry_run {
            log_info("executor", &format!("[DRY-RUN] {}", action));
            return ExecutionResult {
                success: true,
                dry_run: true,
                action,
                details: "Would delete registry value.".to_string(),
            };
        }

        let Some(hive) = Self::get_hive(hive_str) else {
            return ExecutionResult {
                success: false,
                dry_run: false,
                action,
                details: format!("Invalid registry hive: {}", hive_str),
            };
        };

        let path_wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let name_wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        let mut hkey = HKEY::default();

        unsafe {
            let open_res = RegOpenKeyExW(
                hive,
                PCWSTR(path_wide.as_ptr()),
                0,
                KEY_WRITE,
                &mut hkey,
            );

            if open_res != ERROR_SUCCESS {
                return ExecutionResult {
                    success: false,
                    dry_run: false,
                    action,
                    details: "Key does not exist or access denied.".to_string(),
                };
            }

            let del_res = RegDeleteValueW(hkey, PCWSTR(name_wide.as_ptr()));
            let _ = RegCloseKey(hkey);

            if del_res == ERROR_SUCCESS {
                log_info("executor", &format!("Deleted: {}", action));
                ExecutionResult {
                    success: true,
                    dry_run: false,
                    action,
                    details: "Registry value deleted.".to_string(),
                }
            } else {
                ExecutionResult {
                    success: false,
                    dry_run: false,
                    action,
                    details: "Failed to delete value or value does not exist.".to_string(),
                }
            }
        }
    }
}
