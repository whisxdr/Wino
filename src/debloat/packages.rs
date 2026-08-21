use crate::core::logger::{log_error, log_info};
use std::os::windows::process::CommandExt;
use std::process::Command;
use windows::core::PCWSTR;
use windows::Win32::System::Registry::{
    RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ,
};

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn get_installed_appx_packages() -> Vec<String> {
    let mut packages = Vec::new();

    let paths = [
        "Software\\Microsoft\\Windows\\CurrentVersion\\Appx\\AppxAllUserStore\\Applications",
        "Software\\Microsoft\\Windows\\CurrentVersion\\Appx\\AppxAllUserStore\\InboxApplications",
    ];

    for path in paths {
        let path_wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let mut key = HKEY::default();

        unsafe {
            if RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                PCWSTR(path_wide.as_ptr()),
                0,
                KEY_READ,
                &mut key,
            ).is_ok() {
                let mut index = 0u32;
                let mut name_buf = vec![0u16; 512];

                loop {
                    let mut name_len = name_buf.len() as u32;
                    let res = RegEnumKeyExW(
                        key,
                        index,
                        windows::core::PWSTR(name_buf.as_mut_ptr()),
                        &mut name_len,
                        None,
                        windows::core::PWSTR::null(),
                        None,
                        None,
                    );

                    if res.is_err() {
                        break;
                    }

                    let pkg_name = String::from_utf16_lossy(&name_buf[..name_len as usize]);
                    packages.push(pkg_name);
                    index += 1;
                }

                let _ = RegCloseKey(key);
            }
        }
    }

    packages
}

pub fn is_package_installed(package_name_pattern: &str) -> bool {
    let pattern_lower = package_name_pattern.to_lowercase();
    let installed = get_installed_appx_packages();
    installed.iter().any(|p| p.to_lowercase().contains(&pattern_lower))
}

pub fn remove_appx_package(package_name_pattern: &str, dry_run: bool) -> Result<(), String> {
    if dry_run {
        log_info("debloat", &format!("[DRY-RUN] Would remove AppX package: {}", package_name_pattern));
        return Ok(());
    }

    log_info("debloat", &format!("Removing AppX package: {}", package_name_pattern));

    // Controlled, headless package removal with CREATE_NO_WINDOW
    let status = Command::new("powershell.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &format!(
                "Get-AppxPackage -AllUsers -Name *{}* | Remove-AppxPackage -ErrorAction SilentlyContinue",
                package_name_pattern
            ),
        ])
        .status();

    match status {
        Ok(s) if s.success() => {
            log_info("debloat", &format!("Successfully removed package: {}", package_name_pattern));
            Ok(())
        }
        Ok(s) => {
            log_error("debloat", &format!("Failed to remove package {} (Exit code: {:?})", package_name_pattern, s.code()));
            Err(format!("Removal exited with code {:?}", s.code()))
        }
        Err(e) => {
            log_error("debloat", &format!("Failed to spawn process: {}", e));
            Err(e.to_string())
        }
    }
}
