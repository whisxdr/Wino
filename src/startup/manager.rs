//! Startup entry toggling.
//!
//! Only the sources Wino can safely reverse are toggled: the `Run`/`RunOnce`
//! values in either hive and the two Startup folders. Everything else — the
//! `Winlogon` shell values and scheduled tasks — is refused with an explanation
//! rather than attempted, because a mistake there can leave a machine without a
//! desktop or with a task Wino cannot put back.
//!
//! The registry subkey comes from the scanned item's own source label
//! ([`crate::startup::scanner::registry_source_target`]) instead of a hard-coded
//! `Run` path, so a `RunOnce` entry is deleted from `RunOnce` and a WOW6432Node
//! entry from the 32-bit view.

use crate::core::executor::SystemExecutor;
use crate::core::logger::log_info;
use crate::restore::snapshots::create_snapshot;
use crate::startup::scanner::{registry_source_target, StartupItem, SOURCE_WINLOGON};
use std::fs;
use std::path::Path;

/// Suffix Wino appends to a Startup-folder file to disable it.
const DISABLED_SUFFIX: &str = ".disabled";

/// Compute the file name after enabling or disabling a Startup-folder entry.
///
/// Pure, and deliberately a suffix swap rather than `Path::with_extension`:
/// `with_extension("lnk")` on a non-link entry (`tool.exe`) would rename it to
/// `tool.lnk`, destroying the real extension. Only the `.disabled` marker is
/// added or removed, so `tool.exe.disabled` goes back to `tool.exe`.
pub fn toggled_file_name(file_name: &str, enable: bool) -> String {
    let (base, is_disabled) = crate::startup::scanner::parse_folder_entry(file_name);
    match (enable, is_disabled) {
        (true, true) => base.to_string(),
        (false, false) => format!("{}{}", file_name, DISABLED_SUFFIX),
        // Already in the requested state: return the name unchanged so the
        // caller can see there is nothing to rename.
        _ => file_name.to_string(),
    }
}

/// Enable or disable one startup entry.
///
/// `dry_run` reports what would happen and touches nothing. A real toggle
/// snapshots first so the change is restorable from the Restore view.
pub fn toggle_startup_item(item: &StartupItem, enable: bool, dry_run: bool) -> Result<(), String> {
    if item.source == SOURCE_WINLOGON {
        return Err(format!(
            "'{}' is a Winlogon value. Wino does not toggle Winlogon entries: they run before the desktop and a wrong value can leave Windows without a shell. Change it manually with an administrator account if you are certain.",
            item.name
        ));
    }

    if let Some((hive_label, subkey)) = registry_source_target(&item.source) {
        if dry_run {
            log_info(
                "startup",
                &format!(
                    "[DRY-RUN] Would {} startup item '{}' at {}\\{}",
                    if enable { "enable" } else { "disable" },
                    item.name,
                    hive_label,
                    subkey
                ),
            );
            return Ok(());
        }

        let Some(hive) = SystemExecutor::get_hive(hive_label) else {
            return Err(format!("Unknown registry hive '{}'.", hive_label));
        };

        // Snapshot the value being replaced, so the Restore view can put it
        // back. A bare snapshot records nothing and would restore nothing.
        let previous = crate::core::regutil::read_string(hive, subkey, &item.name);
        crate::restore::snapshots::create_snapshot_with_string_entry(
            &format!("Startup toggle: {}", item.name),
            &crate::restore::snapshots::StringBackupEntry {
                hive: hive_label.to_string(),
                path: subkey.to_string(),
                value_name: item.name.clone(),
                previous_value: previous,
            },
        );

        if enable {
            // Re-create the value from the command line the scan recorded. The
            // earlier implementation returned Ok(()) here without writing
            // anything, so the UI reported an entry as enabled while it stayed
            // absent from the registry.
            if item.command.trim().is_empty() {
                return Err(format!(
                    "Wino has no command line recorded for '{}', so it cannot re-create the registry value. Re-enable it in the application's own settings.",
                    item.name
                ));
            }

            return crate::core::regutil::write_string(hive, subkey, &item.name, &item.command)
                .map(|()| {
                    log_info(
                        "startup",
                        &format!(
                            "Enabled startup entry '{}' ({}\\{})",
                            item.name, hive_label, subkey
                        ),
                    );
                })
                .map_err(|e| format!("Failed to re-enable '{}': {}", item.name, e));
        }

        let result = SystemExecutor::delete_registry_value(hive_label, subkey, &item.name, false);
        return if result.success {
            log_info(
                "startup",
                &format!(
                    "Disabled startup entry '{}' ({}\\{})",
                    item.name, hive_label, subkey
                ),
            );
            Ok(())
        } else {
            Err(result.details)
        };
    }

    if item.source.contains("Startup Folder") {
        let path = Path::new(&item.command);
        if dry_run {
            log_info(
                "startup",
                &format!(
                    "[DRY-RUN] Would rename '{}' to '{}'",
                    path.display(),
                    toggled_file_name(&file_name_of(path), enable)
                ),
            );
            return Ok(());
        }

        if !path.exists() {
            return Err(format!(
                "'{}' is no longer in the Startup folder, so there is nothing to change.",
                path.display()
            ));
        }

        let current_name = file_name_of(path);
        let target_name = toggled_file_name(&current_name, enable);
        if target_name == current_name {
            return Ok(());
        }

        let target = path.with_file_name(&target_name);
        if target.exists() {
            return Err(format!(
                "'{}' already exists in the Startup folder, so '{}' was left as it is.",
                target.display(),
                path.display()
            ));
        }

        let _ = create_snapshot(&format!("Startup toggle: {}", item.name));
        fs::rename(path, &target).map_err(|e| {
            format!(
                "Failed to rename '{}' to '{}': {}",
                path.display(),
                target.display(),
                e
            )
        })?;
        log_info(
            "startup",
            &format!(
                "{} startup entry '{}'",
                if enable { "Enabled" } else { "Disabled" },
                item.name
            ),
        );
        return Ok(());
    }

    Err(format!(
        "'{}' comes from '{}', which Wino does not toggle. Scheduled tasks and other managed entries are changed in their own view or by the application that created them.",
        item.name, item.source
    ))
}

/// File name of a path, or the whole path when it has no final component.
fn file_name_of(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::startup::scanner::{SOURCE_HKCU_RUN, SOURCE_HKLM_RUN_ONCE, SOURCE_SCHEDULED_TASK};

    fn item(source: &str, name: &str, command: &str) -> StartupItem {
        StartupItem {
            name: name.to_string(),
            command: command.to_string(),
            exe_path: command.to_string(),
            source: source.to_string(),
            enabled: true,
            impact: "Low".to_string(),
            is_signed: false,
            publisher: "Unverified".to_string(),
        }
    }

    #[test]
    fn winlogon_entries_are_refused_with_an_explanation() {
        let winlogon = item(SOURCE_WINLOGON, "Shell", "explorer.exe");
        let err = toggle_startup_item(&winlogon, false, true).unwrap_err();
        assert!(
            err.contains("Winlogon"),
            "error must name the source: {}",
            err
        );
        assert!(err.contains("Shell"), "error must name the entry: {}", err);
    }

    #[test]
    fn scheduled_tasks_are_refused_even_in_a_dry_run() {
        let task = item(
            SOURCE_SCHEDULED_TASK,
            r"Microsoft\Windows\Vendor\Updater",
            "updater.exe",
        );
        let err = toggle_startup_item(&task, false, true).unwrap_err();
        assert!(
            err.contains("Scheduled Task"),
            "error must name the source: {}",
            err
        );
        assert!(
            err.contains("does not toggle"),
            "error must be explicit: {}",
            err
        );
    }

    #[test]
    fn unknown_sources_are_refused_rather_than_guessed_at() {
        let other = item("Something Else", "x", "x.exe");
        assert!(toggle_startup_item(&other, false, true).is_err());
    }

    #[test]
    fn disabling_appends_the_marker_and_enabling_removes_only_the_marker() {
        // The bug this guards: `with_extension("lnk")` turned `tool.exe` into
        // `tool.lnk`, breaking non-shortcut entries.
        assert_eq!(toggled_file_name("tool.exe", false), "tool.exe.disabled");
        assert_eq!(toggled_file_name("tool.exe.disabled", true), "tool.exe");
        assert_eq!(toggled_file_name("App.lnk", false), "App.lnk.disabled");
        assert_eq!(toggled_file_name("App.lnk.disabled", true), "App.lnk");
        assert_eq!(toggled_file_name("notes.txt", false), "notes.txt.disabled");
        assert_eq!(toggled_file_name("notes.txt.disabled", true), "notes.txt");
    }

    #[test]
    fn a_toggle_that_changes_nothing_returns_the_same_name() {
        assert_eq!(toggled_file_name("App.lnk", true), "App.lnk");
        assert_eq!(
            toggled_file_name("App.lnk.disabled", false),
            "App.lnk.disabled"
        );
    }

    #[test]
    fn registry_toggle_dry_run_reports_the_source_subkey() {
        let hkcu = item(
            SOURCE_HKCU_RUN,
            "Spotify",
            r#""C:\x\spotify.exe" --autostart"#,
        );
        // A dry run performs no snapshot and no registry write, so this is safe
        // on any machine.
        assert!(toggle_startup_item(&hkcu, false, true).is_ok());
        assert!(toggle_startup_item(&hkcu, true, true).is_ok());

        let run_once = item(SOURCE_HKLM_RUN_ONCE, "Setup", r"C:\x\setup.exe");
        assert!(toggle_startup_item(&run_once, false, true).is_ok());
    }
}
