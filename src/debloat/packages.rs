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

/// Resolve the full provisioned package names matching a user-facing pattern
/// (e.g. "Microsoft.ZuneMusic") from the AppxAllUserStore registry enumeration.
pub fn resolve_full_package_names(package_name_pattern: &str) -> Vec<String> {
    let pattern_lower = package_name_pattern.to_lowercase();
    get_installed_appx_packages()
        .into_iter()
        .filter(|p| p.to_lowercase().contains(&pattern_lower))
        .collect()
}

pub fn is_package_installed(package_name_pattern: &str) -> bool {
    !resolve_full_package_names(package_name_pattern).is_empty()
}

pub fn remove_appx_package(package_name_pattern: &str, dry_run: bool) -> Result<(), String> {
    if dry_run {
        log_info("debloat", &format!("[DRY-RUN] Would remove AppX package: {}", package_name_pattern));
        return Ok(());
    }

    // 1. Resolve pattern -> exact full provisioned package name(s) natively via registry.
    let full_names = resolve_full_package_names(package_name_pattern);
    if full_names.is_empty() {
        log_info("debloat", &format!("No installed provisioned package matches '{}'. Treating as already removed.", package_name_pattern));
        return Ok(());
    }

    log_info("debloat", &format!("Removing {} provisioned AppX package(s) via native DISM engine for pattern '{}'", full_names.len(), package_name_pattern));

    let mut any_success = false;
    let mut last_error = String::new();

    for full_name in &full_names {
        // 2. Headless native DISM removal (C++ binary, <10ms spawn, zero PowerShell/.NET overhead).
        let output = Command::new("dism.exe")
            .creation_flags(CREATE_NO_WINDOW)
            .args([
                "/Online",
                "/Remove-ProvisionedAppxPackage",
                &format!("/PackageName:{}", full_name),
                "/NoRestart",
            ])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                log_info("debloat", &format!("DISM removed provisioned package: {}", full_name));
                any_success = true;
            }
            Ok(out) => {
                // DISM exits with 0x800f0043-style codes when the package is not
                // actually provisioned for removal; treat as non-fatal.
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                last_error = if !stderr.is_empty() { stderr } else { stdout };
                log_error("debloat", &format!("DISM failed for {}: {}", full_name, last_error));
            }
            Err(e) => {
                last_error = e.to_string();
                log_error("debloat", &format!("Failed to spawn dism.exe: {}", e));
            }
        }
    }

    if any_success {
        log_info("debloat", &format!("Successfully removed package(s): {}", package_name_pattern));
        Ok(())
    } else {
        Err(format!("DISM removal failed: {}", last_error))
    }
}
