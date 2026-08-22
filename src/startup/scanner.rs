use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use windows::core::PCWSTR;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    RegCloseKey, RegEnumValueW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupItem {
    pub name: String,
    pub command: String,
    pub exe_path: String,
    pub source: String, // "Registry (HKCU)", "Registry (HKLM)", "Startup Folder"
    pub enabled: bool,
    pub impact: String, // "Low", "Medium", "High"
    pub is_signed: bool,
    pub publisher: String,
}

pub fn scan_startup_items() -> Vec<StartupItem> {
    let mut items = Vec::new();

    // 1. Scan HKCU Run
    scan_registry_run(HKEY_CURRENT_USER, "Software\\Microsoft\\Windows\\CurrentVersion\\Run", "Registry (HKCU)", &mut items);

    // 2. Scan HKLM Run
    scan_registry_run(HKEY_LOCAL_MACHINE, "Software\\Microsoft\\Windows\\CurrentVersion\\Run", "Registry (HKLM)", &mut items);

    // 3. Scan User Startup Folder
    if let Some(appdata) = std::env::var_os("APPDATA") {
        let startup_path = PathBuf::from(appdata)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join("Startup");
        scan_folder(&startup_path, "Startup Folder (User)", &mut items);
    }

    // 4. Scan Common Startup Folder
    if let Some(progdata) = std::env::var_os("ProgramData") {
        let common_startup = PathBuf::from(progdata)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join("Startup");
        scan_folder(&common_startup, "Startup Folder (Common)", &mut items);
    }

    items
}

fn scan_registry_run(hive: HKEY, subkey: &str, source_name: &str, items: &mut Vec<StartupItem>) {
    let subkey_wide: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
    let mut hkey = HKEY::default();

    unsafe {
        if RegOpenKeyExW(hive, PCWSTR(subkey_wide.as_ptr()), 0, KEY_READ, &mut hkey) != ERROR_SUCCESS {
            return;
        }

        let mut index = 0u32;
        loop {
            let mut name_buf = vec![0u16; 256];
            let mut name_len = name_buf.len() as u32;
            let mut val_type = 0u32;
            let mut data_buf = vec![0u8; 1024];
            let mut data_len = data_buf.len() as u32;

            let enum_res = RegEnumValueW(
                hkey,
                index,
                windows::core::PWSTR(name_buf.as_mut_ptr()),
                &mut name_len,
                None,
                Some(&mut val_type),
                Some(data_buf.as_mut_ptr()),
                Some(&mut data_len),
            );

            if enum_res != ERROR_SUCCESS {
                break;
            }

            let name = String::from_utf16_lossy(&name_buf[..name_len as usize]).trim().to_string();
            let command = if val_type == REG_SZ.0 {
                let u16_slice = std::slice::from_raw_parts(data_buf.as_ptr() as *const u16, (data_len as usize) / 2);
                String::from_utf16_lossy(u16_slice).trim_matches('\0').trim().to_string()
            } else {
                String::new()
            };

            let exe_path = extract_exe_path(&command);
            let is_signed = if !exe_path.is_empty() {
                crate::security::signatures::is_file_signed(&exe_path)
            } else {
                false
            };

            let impact = estimate_startup_impact(&name, &command);

            items.push(StartupItem {
                name,
                command,
                exe_path,
                source: source_name.to_string(),
                enabled: true,
                impact,
                is_signed,
                publisher: if is_signed { "Verified".to_string() } else { "Unverified".to_string() },
            });

            index += 1;
        }

        let _ = RegCloseKey(hkey);
    }
}

fn scan_folder(folder: &Path, source_name: &str, items: &mut Vec<StartupItem>) {
    if !folder.exists() || !folder.is_dir() {
        return;
    }

    if let Ok(entries) = fs::read_dir(folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let file_name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                if file_name.to_lowercase() == "desktop.ini" {
                    continue;
                }

                let command = path.to_string_lossy().to_string();
                let is_signed = crate::security::signatures::is_file_signed(&command);
                let impact = estimate_startup_impact(&file_name, &command);

                items.push(StartupItem {
                    name: file_name,
                    command: command.clone(),
                    exe_path: command,
                    source: source_name.to_string(),
                    enabled: true,
                    impact,
                    is_signed,
                    publisher: if is_signed { "Verified".to_string() } else { "Unverified".to_string() },
                });
            }
        }
    }
}

fn extract_exe_path(command: &str) -> String {
    let trimmed = command.trim();
    if let Some(stripped) = trimmed.strip_prefix('"') {
        if let Some(end_quote) = stripped.find('"') {
            return stripped[..end_quote].to_string();
        }
    }
    trimmed.split_whitespace().next().unwrap_or("").to_string()
}

fn estimate_startup_impact(name: &str, command: &str) -> String {
    let lower = format!("{} {}", name, command).to_lowercase();
    if lower.contains("discord") || lower.contains("steam") || lower.contains("spotify") || lower.contains("teams") || lower.contains("slack") {
        "High".to_string()
    } else if lower.contains("update") || lower.contains("helper") || lower.contains("cloud") || lower.contains("onedrive") {
        "Medium".to_string()
    } else {
        "Low".to_string()
    }
}
