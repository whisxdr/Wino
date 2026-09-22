//! Application removal.
//!
//! Removal always goes through the application's own uninstaller or the
//! platform's documented removal entry point — Wino never deletes files or
//! registry keys itself, because an application knows what it wrote and Wino
//! does not. A restore point is created first so an unintended removal is
//! recoverable.
//!
//! Exit codes are not treated as a plain success/failure flag: uninstallers
//! routinely return non-zero after succeeding, and `3010`/`1641` specifically
//! mean "removed, reboot required". Reporting those as errors would tell the
//! user the removal failed while the application is already gone.
//!
//! # The one PowerShell use in the Application Manager
//!
//! `Remove-AppxPackage` is the documented removal path for an AppX/MSIX
//! package. The `windows` 0.58 crate exposes no Appx deployment
//! (`PackageManager`) COM interface — `Win32_System_ApplicationInstallationAndServicing`
//! covers Windows Installer and MSI, not AppX — so there is no native call to
//! make, and hand-declaring the deployment COM vtable is a large, fragile
//! surface for a single operation. This module is therefore the only place in
//! Wino that starts PowerShell, it is confined to that one command, and every
//! other removal path here is a direct process launch.

use crate::apps::models::{split_command_line, AppRecord, UninstallMethod};
use crate::core::logger::{log_info, log_warn};
use crate::core::proc;

/// `ERROR_SUCCESS_REBOOT_REQUIRED`: removal succeeded, a restart completes it.
const MSI_REBOOT_REQUIRED: i32 = 3010;
/// `ERROR_SUCCESS_REBOOT_INITIATED`: removal succeeded, a restart was started.
const MSI_REBOOT_INITIATED: i32 = 1641;

/// Remove one application.
///
/// `dry_run` returns the command that would run without spawning anything.
pub fn uninstall_app(record: &AppRecord, dry_run: bool) -> Result<String, String> {
    let Some(method) = record.uninstall.as_ref() else {
        return Err(format!(
            "'{}' exposes no uninstall method, so Wino cannot remove it.",
            record.display_name
        ));
    };

    if dry_run {
        return Ok(format!(
            "Would uninstall '{}' using {} (no change made).",
            record.display_name,
            method.kind_label()
        ));
    }

    crate::restore::snapshots::create_snapshot(&format!("Uninstall: {}", record.display_name));

    match method {
        UninstallMethod::RegistryCommand { command } => {
            run_registry_uninstaller(&record.display_name, command)
        }
        UninstallMethod::RegistryQuietCommand { command } => {
            run_registry_uninstaller(&record.display_name, command)
        }
        UninstallMethod::MsiProductCode { product_code } => {
            run_msi_uninstall(&record.display_name, product_code)
        }
        UninstallMethod::AppxPackage { full_name } => {
            run_appx_uninstall(&record.display_name, full_name)
        }
        UninstallMethod::WingetPackage { package_id } => {
            run_winget_uninstall(&record.display_name, package_id)
        }
    }
}

/// Launch the vendor's own uninstaller, directly rather than through a shell so
/// no command string is reinterpreted.
fn run_registry_uninstaller(display_name: &str, command: &str) -> Result<String, String> {
    let Some((exe, args)) = split_command_line(command) else {
        return Err(format!(
            "'{}' has an unreadable uninstall command.",
            display_name
        ));
    };

    let arg_refs: Vec<&str> = args.split_whitespace().collect();
    let out = proc::run(&exe, &arg_refs)?;
    finish(display_name, "uninstaller", &out)
}

/// Remove an MSI product through `msiexec.exe`.
fn run_msi_uninstall(display_name: &str, product_code: &str) -> Result<String, String> {
    let code = product_code.trim();
    if code.is_empty() {
        return Err(format!(
            "'{}' has no MSI product code recorded.",
            display_name
        ));
    }

    let out = proc::run("msiexec.exe", &["/x", code, "/qn", "/norestart"])?;
    finish(display_name, "msiexec", &out)
}

/// Remove an AppX/MSIX package. See the module comment for why PowerShell is
/// used here and nowhere else in this module.
fn run_appx_uninstall(display_name: &str, full_name: &str) -> Result<String, String> {
    let package = full_name.trim();
    if package.is_empty() {
        return Err(format!("'{}' has no package name recorded.", display_name));
    }
    // The name is interpolated into a PowerShell command, so it must not be
    // able to break out of the string literal. Real package names never contain
    // quotes or whitespace.
    if package.contains('\'') || package.contains('"') || package.chars().any(char::is_whitespace) {
        return Err(format!(
            "'{}' has a package name Wino will not pass to a shell: {}",
            display_name, package
        ));
    }

    let script = format!("Remove-AppxPackage -Package '{}'", package);
    let out = proc::run(
        "powershell.exe",
        &["-NoProfile", "-NonInteractive", "-Command", &script],
    )?;
    finish(display_name, "Remove-AppxPackage", &out)
}

/// Remove a package through winget, the same interface used for updates.
fn run_winget_uninstall(display_name: &str, package_id: &str) -> Result<String, String> {
    let id = package_id.trim();
    if id.is_empty() {
        return Err(format!(
            "'{}' has no Winget package id recorded.",
            display_name
        ));
    }

    let out = proc::run(
        "winget.exe",
        &[
            "uninstall",
            "--id",
            id,
            "--silent",
            "--accept-source-agreements",
            "--disable-interactivity",
        ],
    )?;
    finish(display_name, "winget", &out)
}

/// Turn a finished removal process into the message the UI shows.
fn finish(display_name: &str, tool: &str, out: &proc::ToolOutput) -> Result<String, String> {
    if out.success {
        log_info(
            "apps",
            &format!("Uninstalled '{}' via {}", display_name, tool),
        );
        return Ok(format!("'{}' was uninstalled.", display_name));
    }

    match out.exit_code {
        Some(code @ (MSI_REBOOT_REQUIRED | MSI_REBOOT_INITIATED)) => {
            log_info(
                "apps",
                &format!(
                    "Uninstalled '{}' via {} (exit {})",
                    display_name, tool, code
                ),
            );
            Ok(format!(
                "'{}' was uninstalled. Windows must be restarted to finish removing it.",
                display_name
            ))
        }
        code => {
            let detail = out.last_line();
            log_warn(
                "apps",
                &format!(
                    "Uninstall of '{}' via {} failed: {}",
                    display_name, tool, detail
                ),
            );
            Err(format!(
                "Uninstalling '{}' failed ({} exited with {}): {}",
                display_name,
                tool,
                code.map(|c| c.to_string())
                    .unwrap_or_else(|| "no code".to_string()),
                if detail.is_empty() {
                    "no output".to_string()
                } else {
                    detail
                }
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apps::models::AppSource;

    fn record(uninstall: Option<UninstallMethod>) -> AppRecord {
        AppRecord {
            name: "Contoso".to_string(),
            display_name: "Contoso Editor".to_string(),
            publisher: "Contoso Ltd".to_string(),
            version: "1.0".to_string(),
            install_location: String::new(),
            install_size: None,
            install_date: String::new(),
            package_type: "EXE".to_string(),
            source: AppSource::Win32,
            signed: None,
            uninstall,
            provisioned: false,
            winget_id: None,
        }
    }

    #[test]
    fn dry_run_never_spawns_and_names_the_application() {
        let rec = record(Some(UninstallMethod::RegistryCommand {
            command: r"C:\Program Files\Contoso\unins000.exe /S".to_string(),
        }));
        let msg = uninstall_app(&rec, true).expect("dry run succeeds");
        assert!(msg.contains("Contoso Editor"));
        assert!(msg.contains("UninstallString"));
        assert!(msg.contains("no change made"));
    }

    #[test]
    fn record_without_uninstall_method_is_refused() {
        let rec = record(None);
        let err = uninstall_app(&rec, true).expect_err("no method");
        assert!(err.contains("Contoso Editor"));
        assert!(err.contains("no uninstall method"));
    }

    #[test]
    fn reboot_exit_codes_are_success_with_a_note() {
        let out = proc::ToolOutput {
            success: false,
            exit_code: Some(3010),
            stdout: String::new(),
            stderr: "The requested operation is successful. Changes will not be effective until the system is rebooted.".to_string(),
        };
        let msg = finish("Contoso Editor", "msiexec", &out).expect("3010 is success");
        assert!(msg.contains("restarted"));

        let initiated = proc::ToolOutput {
            success: false,
            exit_code: Some(1641),
            stdout: String::new(),
            stderr: String::new(),
        };
        let msg = finish("Contoso Editor", "msiexec", &initiated).expect("1641 is success");
        assert!(msg.contains("restarted"));
    }

    #[test]
    fn real_failure_carries_the_tool_output() {
        let out = proc::ToolOutput {
            success: false,
            exit_code: Some(1603),
            stdout: String::new(),
            stderr: "Fatal error during installation.".to_string(),
        };
        let err = finish("Contoso Editor", "msiexec", &out).expect_err("1603 is a failure");
        assert!(err.contains("Contoso Editor"));
        assert!(err.contains("1603"));
        assert!(err.contains("Fatal error during installation."));
    }
}
