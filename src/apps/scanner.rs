//! Installed application discovery.
//!
//! Everything here is read through the native registry API (`regutil` and
//! `SystemExecutor`), never through a shell. The Application Manager scan runs
//! on a worker thread while the UI is already up, so spawning a script host per
//! query would add seconds of latency and a string-injection surface for no
//! information a direct registry read cannot provide.
//!
//! Two ceilings are deliberate and load-bearing:
//!
//! * at most `MAX_RECORDS` records are collected, so a machine with a
//!   pathological `Uninstall` tree cannot make the scan allocate without bound;
//! * at most `MAX_SIGNATURE_CHECKS` Authenticode verifications run, because
//!   `WinVerifyTrust` is the only slow call in the scan and `signed` is a
//!   display hint, not an input to any decision.

use std::collections::HashSet;

use windows::Win32::System::Registry::{HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

use crate::apps::models::{
    classify_package_type, classify_uninstall_method, split_command_line, AppRecord, AppSource,
    UninstallMethod,
};
use crate::core::executor::SystemExecutor;
use crate::core::logger::{log_info, log_warn};
use crate::core::regutil;

/// Hard cap on records collected per scan.
const MAX_RECORDS: usize = 4000;

/// Hard cap on Authenticode verifications per scan.
const MAX_SIGNATURE_CHECKS: usize = 200;

const UNINSTALL_PATH: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall";
const UNINSTALL_PATH_WOW: &str =
    "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall";
const APPX_APPLICATIONS: &str =
    "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Appx\\AppxAllUserStore\\Applications";
const APPX_INBOX: &str =
    "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Appx\\AppxAllUserStore\\InboxApplications";

/// Enumerate every installed application: Win32 programs from the machine and
/// user `Uninstall` keys, then Store and provisioned AppX packages.
pub fn scan_installed_apps() -> Vec<AppRecord> {
    let mut apps = Vec::with_capacity(256);
    let mut signature_budget = MAX_SIGNATURE_CHECKS;
    let mut capped = false;

    scan_uninstall_root(
        HKEY_LOCAL_MACHINE,
        "HKLM",
        UNINSTALL_PATH,
        &mut apps,
        &mut signature_budget,
        &mut capped,
    );
    scan_uninstall_root(
        HKEY_LOCAL_MACHINE,
        "HKLM",
        UNINSTALL_PATH_WOW,
        &mut apps,
        &mut signature_budget,
        &mut capped,
    );
    scan_uninstall_root(
        HKEY_CURRENT_USER,
        "HKCU",
        UNINSTALL_PATH,
        &mut apps,
        &mut signature_budget,
        &mut capped,
    );
    scan_appx_packages(&mut apps, &mut capped);

    if capped {
        log_warn(
            "apps",
            &format!(
                "Application scan stopped at {} records; the remaining entries were not read",
                MAX_RECORDS
            ),
        );
    }

    apps.sort_by_cached_key(|a| a.sort_key());
    log_info(
        "apps",
        &format!("Application scan found {} records", apps.len()),
    );
    apps
}

/// Read one `Uninstall` root: every sub-key is one installed program.
fn scan_uninstall_root(
    hive: HKEY,
    hive_label: &str,
    root: &str,
    apps: &mut Vec<AppRecord>,
    signature_budget: &mut usize,
    capped: &mut bool,
) {
    for key_name in regutil::enum_subkeys(hive, root) {
        if apps.len() >= MAX_RECORDS {
            *capped = true;
            return;
        }

        let path = format!("{}\\{}", root, key_name);
        let display_name = read_value(hive, &path, "DisplayName");
        let system_component =
            SystemExecutor::read_registry_dword(hive_label, &path, "SystemComponent");
        if should_skip_entry(&display_name, system_component) {
            continue;
        }

        let uninstall_string = read_value(hive, &path, "UninstallString");
        let quiet_string = read_value(hive, &path, "QuietUninstallString");
        let product_code = if is_guid_key(&key_name) {
            key_name.clone()
        } else {
            String::new()
        };
        let windows_installer =
            SystemExecutor::read_registry_dword(hive_label, &path, "WindowsInstaller") == Some(1);
        let is_msi = !product_code.is_empty() || windows_installer;

        // `EstimatedSize` is documented in KB.
        let install_size = SystemExecutor::read_registry_dword(hive_label, &path, "EstimatedSize")
            .map(|kb| u64::from(kb) * 1024);

        apps.push(AppRecord {
            name: key_name.clone(),
            display_name,
            publisher: read_value(hive, &path, "Publisher"),
            version: read_value(hive, &path, "DisplayVersion"),
            install_location: read_value(hive, &path, "InstallLocation"),
            install_size,
            install_date: read_value(hive, &path, "InstallDate"),
            package_type: classify_package_type(&key_name, &uninstall_string, is_msi, false),
            source: AppSource::Win32,
            signed: resolve_signature(&quiet_string, &uninstall_string, signature_budget),
            uninstall: classify_uninstall_method(
                &quiet_string,
                &uninstall_string,
                &product_code,
                "",
            ),
            provisioned: false,
            winget_id: None,
        });
    }
}

/// Enumerate installed Store packages, then provisioned packages that are not
/// installed for anyone yet.
fn scan_appx_packages(apps: &mut Vec<AppRecord>, capped: &mut bool) {
    let installed = regutil::enum_subkeys(HKEY_LOCAL_MACHINE, APPX_APPLICATIONS);
    let inbox = regutil::enum_subkeys(HKEY_LOCAL_MACHINE, APPX_INBOX);
    let installed_names: HashSet<&str> = installed.iter().map(String::as_str).collect();

    for full_name in &installed {
        if apps.len() >= MAX_RECORDS {
            *capped = true;
            return;
        }
        apps.push(appx_record(full_name, AppSource::MicrosoftStore, false));
    }

    for full_name in &inbox {
        // A package present in both keys is already installed and was recorded
        // above; counting it twice would show the same app under two filters.
        if installed_names.contains(full_name.as_str()) {
            continue;
        }
        if apps.len() >= MAX_RECORDS {
            *capped = true;
            return;
        }
        apps.push(appx_record(full_name, AppSource::AppX, true));
    }
}

fn appx_record(full_name: &str, source: AppSource, provisioned: bool) -> AppRecord {
    let (display_name, version, publisher) = match parse_appx_full_name(full_name) {
        Some(identity) => (identity.name, identity.version, identity.publisher_id),
        // A malformed package name is still a real installed package, so it is
        // listed verbatim rather than dropped.
        None => (full_name.to_string(), String::new(), String::new()),
    };

    AppRecord {
        name: full_name.to_string(),
        display_name,
        publisher,
        version,
        install_location: String::new(),
        install_size: None,
        install_date: String::new(),
        package_type: "AppX".to_string(),
        source,
        // MSIX payloads are catalog-signed by the Store, not Authenticode-signed
        // on disk, so a file signature check would report a false negative.
        signed: None,
        uninstall: Some(UninstallMethod::AppxPackage {
            full_name: full_name.to_string(),
        }),
        provisioned,
        winget_id: None,
    }
}

fn read_value(hive: HKEY, path: &str, value_name: &str) -> String {
    // ponytail: regutil caps values at 4 KiB; an oversized UninstallString is
    // reported as absent. Raise the cap in regutil if a vendor ever exceeds it.
    regutil::read_string(hive, path, value_name)
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// Authenticode verdict for the uninstaller executable, when one is resolvable
/// and the verification budget is not spent.
fn resolve_signature(quiet: &str, plain: &str, budget: &mut usize) -> Option<bool> {
    if *budget == 0 {
        return None;
    }
    let command = if quiet.trim().is_empty() {
        plain
    } else {
        quiet
    };
    let (exe, _args) = split_command_line(command)?;
    if !std::path::Path::new(&exe).exists() {
        return None;
    }
    *budget -= 1;
    Some(crate::security::signatures::is_file_signed(&exe))
}

/// Whether a registry entry describes a user-facing application.
///
/// `SystemComponent = 1` marks runtimes, redistributables and driver helpers
/// that Windows hides from "Apps & features" — Wino hides them too.
fn should_skip_entry(display_name: &str, system_component: Option<u32>) -> bool {
    display_name.trim().is_empty() || system_component == Some(1)
}

/// Whether an `Uninstall` sub-key name is an MSI product code (`{GUID}`).
fn is_guid_key(key_name: &str) -> bool {
    let key = key_name.trim();
    if key.len() != 38 || !key.starts_with('{') || !key.ends_with('}') {
        return false;
    }
    let inner = &key[1..key.len() - 1];
    inner.len() == 36 && inner.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

/// Identity parsed out of an AppX package full name.
#[derive(Debug, PartialEq, Eq)]
struct AppxIdentity {
    name: String,
    version: String,
    publisher_id: String,
}

/// Parse `Name_Version_Architecture[_ResourceId]_PublisherId`.
///
/// The resource-id segment is empty for most packages
/// (`Microsoft.ZuneMusic_11.2403.2.0_x64__8wekyb3d8bbwe`), which is why the
/// publisher id is read as the last segment rather than at a fixed index.
fn parse_appx_full_name(full_name: &str) -> Option<AppxIdentity> {
    let parts: Vec<&str> = full_name.trim().split('_').collect();
    if parts.len() < 3 {
        return None;
    }

    let name = parts[0];
    let version = parts[1];
    let publisher_id = parts[parts.len() - 1];
    if name.is_empty() || version.is_empty() || publisher_id.is_empty() {
        return None;
    }

    Some(AppxIdentity {
        name: name.to_string(),
        version: version.to_string(),
        publisher_id: publisher_id.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appx_full_name_parses_empty_resource_id() {
        let identity = parse_appx_full_name("Microsoft.ZuneMusic_11.2403.2.0_x64__8wekyb3d8bbwe")
            .expect("zune music package name");
        assert_eq!(identity.name, "Microsoft.ZuneMusic");
        assert_eq!(identity.version, "11.2403.2.0");
        assert_eq!(identity.publisher_id, "8wekyb3d8bbwe");
    }

    #[test]
    fn appx_full_name_parses_non_empty_resource_id_and_leading_whitespace() {
        let identity = parse_appx_full_name(
            " Microsoft.WindowsCalculator_11.2210.0.0_x64_en-US_8wekyb3d8bbwe ",
        )
        .expect("calculator package name");
        assert_eq!(identity.name, "Microsoft.WindowsCalculator");
        assert_eq!(identity.version, "11.2210.0.0");
        assert_eq!(identity.publisher_id, "8wekyb3d8bbwe");
    }

    #[test]
    fn appx_full_name_rejects_unshaped_input() {
        assert!(parse_appx_full_name("Contoso.App").is_none());
        assert!(parse_appx_full_name("Contoso.App_1.0").is_none());
        assert!(parse_appx_full_name("Contoso.App__x64_abc").is_none());
        assert!(parse_appx_full_name("").is_none());
    }

    #[test]
    fn entries_without_display_name_or_with_system_component_are_skipped() {
        assert!(should_skip_entry("", None));
        assert!(should_skip_entry("   ", Some(0)));
        assert!(should_skip_entry("Visual C++ Runtime", Some(1)));
        assert!(!should_skip_entry("Contoso Editor", None));
        assert!(!should_skip_entry("Contoso Editor", Some(0)));
    }

    #[test]
    fn msi_product_codes_are_detected_by_shape() {
        assert!(is_guid_key("{4F6C3B2E-9A11-4B3D-9C2F-1D2E3A4B5C6D}"));
        assert!(is_guid_key("  {4f6c3b2e-9a11-4b3d-9c2f-1d2e3a4b5c6d}  "));
        assert!(!is_guid_key("{4F6C3B2E-9A11-4B3D-9C2F-1D2E3A4B5C6D"));
        assert!(!is_guid_key("{4F6C3B2E-9A11-4B3D-9C2F-1D2E3A4B5C6}"));
        assert!(!is_guid_key("{ZZZZZZZZ-9A11-4B3D-9C2F-1D2E3A4B5C6D}"));
        assert!(!is_guid_key("Contoso Editor"));
        assert!(!is_guid_key(""));
    }
}
