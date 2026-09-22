//! Application list presentation: filtering, sorting, update correlation, and
//! opening an install location.
//!
//! Filtering and sorting run on every frame against a list that can hold
//! thousands of records, so they are pure functions over borrowed slices: no
//! cloning of the record list, and the view decides when to re-run them.

use std::cmp::Ordering;
use std::ptr;

use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use crate::apps::models::{AppRecord, AppSort, AppSource, WingetUpdate};

/// Filter by substring and source. `query_lower` must already be lowercase —
/// the caller lowercases once per frame instead of once per record.
pub fn filter_apps<'a>(
    apps: &'a [AppRecord],
    query_lower: &str,
    source: Option<AppSource>,
) -> Vec<&'a AppRecord> {
    apps.iter()
        .filter(|a| source.map(|s| a.source == s).unwrap_or(true))
        .filter(|a| a.matches(query_lower))
        .collect()
}

/// Sort in place. Name and Publisher are ascending; Size, InstallDate and
/// Version are descending, because the newest or largest entry is what the user
/// is looking for when they pick those columns.
pub fn sort_apps(apps: &mut [AppRecord], sort: AppSort) {
    match sort {
        AppSort::Name => apps.sort_by_key(|a| a.sort_key()),
        AppSort::Publisher => apps.sort_by(|a, b| {
            a.publisher
                .to_lowercase()
                .cmp(&b.publisher.to_lowercase())
                .then_with(|| a.sort_key().cmp(&b.sort_key()))
        }),
        // Records with no recorded size sort last: an unknown size is not a
        // zero size, and putting them first would bury the real entries.
        AppSort::Size => apps.sort_by(|a, b| match (a.install_size, b.install_size) {
            (Some(x), Some(y)) => y.cmp(&x),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }),
        AppSort::InstallDate => apps.sort_by(|a, b| {
            let (a, b) = (a.install_date.trim(), b.install_date.trim());
            match (a.is_empty(), b.is_empty()) {
                (false, false) => b.cmp(a),
                (false, true) => Ordering::Less,
                (true, false) => Ordering::Greater,
                (true, true) => Ordering::Equal,
            }
        }),
        AppSort::Version => apps.sort_by(|a, b| compare_versions(&b.version, &a.version)),
    }
}

/// Compare two version strings component-wise.
///
/// Dot-separated numeric components compare numerically, so `1.10` sorts above
/// `1.9` — the plain string comparison that a naive implementation would use
/// gets that backwards. Differing component counts compare on the common prefix
/// first (`1.2` < `1.2.1`), and a non-numeric component falls back to a
/// case-insensitive string comparison so vendor schemes like `1.0-beta` still
/// order deterministically instead of collapsing to Equal.
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let mut left = a.trim().split('.');
    let mut right = b.trim().split('.');

    loop {
        match (left.next(), right.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => {
                let order = match (x.trim().parse::<u64>(), y.trim().parse::<u64>()) {
                    (Ok(nx), Ok(ny)) => nx.cmp(&ny),
                    _ => x.trim().to_lowercase().cmp(&y.trim().to_lowercase()),
                };
                if order != Ordering::Equal {
                    return order;
                }
            }
        }
    }
}

/// Attach each update to the application record it belongs to.
///
/// Matching prefers the winget id, which is exact, then falls back to a
/// case-insensitive display-name comparison for packages winget knows under a
/// name the registry recorded differently. An unmatched update keeps
/// `app_name = None` and is still shown — winget reports upgrades for packages
/// Wino did not enumerate, and hiding them would understate what is available.
pub fn correlate_updates(apps: &[AppRecord], updates: &[WingetUpdate]) -> Vec<WingetUpdate> {
    updates
        .iter()
        .map(|update| {
            let mut correlated = update.clone();
            correlated.app_name = find_match(apps, update).map(|a| a.display_name.clone());
            correlated
        })
        .collect()
}

fn find_match<'a>(apps: &'a [AppRecord], update: &WingetUpdate) -> Option<&'a AppRecord> {
    let id = update.package_id.trim();
    if !id.is_empty() {
        if let Some(found) = apps.iter().find(|a| {
            a.winget_id
                .as_deref()
                .map(|w| w.eq_ignore_ascii_case(id))
                .unwrap_or(false)
        }) {
            return Some(found);
        }
    }

    let name = update.name.trim();
    if name.is_empty() {
        return None;
    }
    apps.iter()
        .find(|a| a.display_name.trim().eq_ignore_ascii_case(name))
}

/// Open Explorer at the application's install location.
///
/// `ShellExecuteW` with the `open` verb on a directory is the shell's own
/// behaviour for a folder path, so it reuses the user's file manager instead of
/// hard-coding `explorer.exe`.
pub fn open_install_location(record: &AppRecord) -> Result<(), String> {
    let location = record.install_location.trim();
    if location.is_empty() {
        return Err(format!(
            "'{}' does not record an install location.",
            record.display_name
        ));
    }
    if !std::path::Path::new(location).exists() {
        return Err(format!(
            "The recorded install location for '{}' no longer exists: {}",
            record.display_name, location
        ));
    }

    let verb = crate::core::regutil::wide("open");
    let path = crate::core::regutil::wide(location);

    let result = unsafe {
        ShellExecuteW(
            HWND(ptr::null_mut()),
            PCWSTR(verb.as_ptr()),
            PCWSTR(path.as_ptr()),
            PCWSTR(ptr::null()),
            PCWSTR(ptr::null()),
            SW_SHOWNORMAL,
        )
    };

    // ShellExecuteW reports failure as a pseudo-HANDLE at or below 32.
    if result.0 as usize <= 32 {
        return Err(format!("Windows could not open {}.", location));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apps::models::{UninstallMethod, UpdateState};

    fn app(name: &str, publisher: &str, version: &str, size: Option<u64>, date: &str) -> AppRecord {
        AppRecord {
            name: name.to_string(),
            display_name: name.to_string(),
            publisher: publisher.to_string(),
            version: version.to_string(),
            install_location: String::new(),
            install_size: size,
            install_date: date.to_string(),
            package_type: "EXE".to_string(),
            source: AppSource::Win32,
            signed: None,
            uninstall: Some(UninstallMethod::RegistryCommand {
                command: "unins000.exe".to_string(),
            }),
            provisioned: false,
            winget_id: None,
        }
    }

    fn update(id: &str, name: &str) -> WingetUpdate {
        WingetUpdate {
            package_id: id.to_string(),
            name: name.to_string(),
            current_version: "1.0".to_string(),
            available_version: "2.0".to_string(),
            app_name: None,
            state: UpdateState::UpdateAvailable,
        }
    }

    #[test]
    fn filter_applies_query_and_source_together() {
        let mut win32 = app("Contoso Editor", "Contoso Ltd", "1.0", None, "");
        win32.source = AppSource::Win32;
        let mut store = app("Contoso Notes", "Contoso Ltd", "1.0", None, "");
        store.source = AppSource::MicrosoftStore;
        let other = app("Fabrikam Tool", "Fabrikam", "1.0", None, "");
        let apps = vec![win32, store, other];

        assert_eq!(filter_apps(&apps, "contoso", None).len(), 2);
        assert_eq!(
            filter_apps(&apps, "contoso", Some(AppSource::Win32)).len(),
            1
        );
        assert_eq!(
            filter_apps(&apps, "contoso", Some(AppSource::MicrosoftStore)).len(),
            1
        );
        assert_eq!(filter_apps(&apps, "", None).len(), 3);
        assert!(filter_apps(&apps, "nothing", None).is_empty());
    }

    #[test]
    fn version_comparison_is_numeric_not_lexical() {
        assert_eq!(compare_versions("1.10", "1.9"), Ordering::Greater);
        assert_eq!(compare_versions("1.9", "1.10"), Ordering::Less);
        assert_eq!(compare_versions("1.2", "1.2.0"), Ordering::Less);
        assert_eq!(compare_versions("1.2.0.1", "1.2"), Ordering::Greater);
        assert_eq!(compare_versions("2.0", "2.0"), Ordering::Equal);
        assert_eq!(
            compare_versions("11.2403.2.0", "11.2403.10.0"),
            Ordering::Less
        );
        assert_eq!(compare_versions("", ""), Ordering::Equal);
        assert_eq!(compare_versions("1.0-beta", "1.0-alpha"), Ordering::Greater);
    }

    #[test]
    fn size_sort_is_descending_with_unknown_last() {
        let mut apps = vec![
            app("Small", "P", "1.0", Some(1024), ""),
            app("Unknown", "P", "1.0", None, ""),
            app("Large", "P", "1.0", Some(4096), ""),
        ];
        sort_apps(&mut apps, AppSort::Size);
        assert_eq!(apps[0].display_name, "Large");
        assert_eq!(apps[1].display_name, "Small");
        assert_eq!(apps[2].display_name, "Unknown");
    }

    #[test]
    fn install_date_sort_is_descending_with_empty_last() {
        let mut apps = vec![
            app("Old", "P", "1.0", None, "20230101"),
            app("NoDate", "P", "1.0", None, ""),
            app("New", "P", "1.0", None, "20240115"),
        ];
        sort_apps(&mut apps, AppSort::InstallDate);
        assert_eq!(apps[0].display_name, "New");
        assert_eq!(apps[1].display_name, "Old");
        assert_eq!(apps[2].display_name, "NoDate");
    }

    #[test]
    fn name_sort_is_case_insensitive() {
        let mut apps = vec![
            app("zebra", "P", "1.0", None, ""),
            app("Alpha", "P", "1.0", None, ""),
        ];
        sort_apps(&mut apps, AppSort::Name);
        assert_eq!(apps[0].display_name, "Alpha");
        assert_eq!(apps[1].display_name, "zebra");
    }

    #[test]
    fn correlation_prefers_winget_id_then_display_name() {
        let mut by_id = app("Contoso Editor", "Contoso Ltd", "1.0", None, "");
        by_id.winget_id = Some("Contoso.Editor".to_string());
        let by_name = app("Fabrikam Tool", "Fabrikam", "1.0", None, "");
        let apps = vec![by_id, by_name];

        let updates = vec![
            update("Contoso.Editor", "Contoso Editor"),
            update("Fabrikam.Tool", "fabrikam tool"),
            update("Nobody.Package", "Nobody Package"),
        ];

        let correlated = correlate_updates(&apps, &updates);
        assert_eq!(correlated[0].app_name.as_deref(), Some("Contoso Editor"));
        assert_eq!(correlated[1].app_name.as_deref(), Some("Fabrikam Tool"));
        assert!(correlated[2].app_name.is_none());
        assert_eq!(correlated[2].state, UpdateState::UpdateAvailable);
    }

    #[test]
    fn open_install_location_rejects_empty_and_missing_paths() {
        let empty = app("Contoso", "P", "1.0", None, "");
        let err = open_install_location(&empty).expect_err("empty location");
        assert!(err.contains("does not record an install location"));

        let mut missing = app("Contoso", "P", "1.0", None, "");
        missing.install_location = r"C:\Wino\Definitely\Not\Here".to_string();
        let err = open_install_location(&missing).expect_err("missing path");
        assert!(err.contains("no longer exists"));
    }
}
