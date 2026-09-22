//! Winget integration.
//!
//! Winget is a third-party package manager that ships only as a command-line
//! tool, so a subprocess is the correct and only interface. It is strictly
//! **optional**: Wino works fully without it. [`winget_status`] reports whether
//! it exists, [`check_updates`] returns an empty list when it does not, and
//! nothing in this module may panic or block Wino's startup — every entry point
//! degrades to a "not available" result instead.
//!
//! Every invocation passes `--disable-interactivity`, so winget can never stop
//! on an interactive prompt waiting for a console that `CREATE_NO_WINDOW` does
//! not provide. Without that flag a hidden prompt would hang the worker thread
//! indefinitely.

use crate::apps::models::{UpdateState, WingetStatus, WingetUpdate};
use crate::core::logger::{log_info, log_warn};
use crate::core::proc;

/// Upper bound on parsed upgrade rows.
const MAX_UPGRADE_ROWS: usize = 2000;

/// Arguments applied to every winget invocation.
const COMMON_ARGS: [&str; 2] = ["--accept-source-agreements", "--disable-interactivity"];

/// Whether winget is installed and usable.
///
/// Never fails: a missing or broken winget produces a [`WingetStatus::missing`]
/// carrying the reason, which the UI shows instead of a control that cannot
/// work.
pub fn winget_status() -> WingetStatus {
    match proc::run("winget.exe", &["--version"]) {
        Ok(out) if out.success => WingetStatus {
            available: true,
            version: out.last_line(),
            detail: String::new(),
        },
        Ok(out) => {
            let detail = out.last_line();
            log_warn(
                "apps",
                &format!(
                    "winget --version failed with {}: {}",
                    out.exit_code
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "no code".to_string()),
                    if detail.is_empty() {
                        "no output".to_string()
                    } else {
                        detail
                    }
                ),
            );
            WingetStatus::missing(
                "winget.exe was found but did not report a version. Repair App Installer from the Microsoft Store.",
            )
        }
        Err(e) => {
            log_warn("apps", &format!("winget is unavailable: {}", e));
            WingetStatus::missing(
                "winget.exe was not found on PATH. Install App Installer from the Microsoft Store. Wino works without it; only update checks and installs need it.",
            )
        }
    }
}

/// Query winget for packages that have a newer version available.
///
/// Returns an empty list when winget is unavailable or reports no upgrades —
/// the two cases are distinguishable through [`winget_status`], which the UI
/// calls separately.
pub fn check_updates() -> Vec<WingetUpdate> {
    let mut args = vec!["upgrade", "--include-unknown"];
    args.extend_from_slice(&COMMON_ARGS);

    let out = match proc::run("winget.exe", &args) {
        Ok(out) => out,
        Err(e) => {
            log_warn(
                "apps",
                &format!("winget upgrade query failed to start: {}", e),
            );
            return Vec::new();
        }
    };

    // winget exits non-zero both for "no upgrades available" and for source
    // failures, so the table is parsed either way; an empty parse simply means
    // there is nothing to offer.
    let updates = parse_upgrade_table(&out.combined());
    if !out.success && updates.is_empty() {
        log_warn(
            "apps",
            &format!("winget upgrade reported: {}", out.last_line()),
        );
    }
    updates
}

/// Parse the fixed-column `winget upgrade` table.
///
/// Columns are padded, so fields are separated by runs of two or more spaces.
/// The four-field requirement is the real filter: the banner lines, the warning
/// lines winget prints before the table, the column header, the dash rule, the
/// "requires explicit targeting" help text and the trailing
/// "N upgrades available." summary all fail it. Rows are never guessed at,
/// because a fabricated row would offer an update for a package that does not
/// exist.
pub fn parse_upgrade_table(output: &str) -> Vec<WingetUpdate> {
    let mut updates = Vec::new();

    for line in output.lines() {
        if updates.len() >= MAX_UPGRADE_ROWS {
            log_warn(
                "apps",
                &format!("Winget upgrade list truncated at {} rows", MAX_UPGRADE_ROWS),
            );
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() || is_header_or_separator(trimmed) || is_count_line(trimmed) {
            continue;
        }

        let fields = split_columns(trimmed);
        if fields.len() < 4 {
            continue;
        }

        let package_id = fields[1].trim();
        if package_id.is_empty() {
            continue;
        }

        updates.push(WingetUpdate {
            package_id: package_id.to_string(),
            name: fields[0].trim().to_string(),
            current_version: fields[2].trim().to_string(),
            available_version: fields[3].trim().to_string(),
            app_name: None,
            state: UpdateState::UpdateAvailable,
        });
    }

    updates
}

/// Whether a line is the column header or its dash rule.
fn is_header_or_separator(line: &str) -> bool {
    let fields = split_columns(line);
    if fields.len() >= 2
        && fields[0].trim().eq_ignore_ascii_case("name")
        && fields[1].trim().eq_ignore_ascii_case("id")
    {
        return true;
    }
    !line.is_empty() && line.chars().all(|c| c == '-' || c.is_whitespace())
}

/// Whether a line is winget's trailing "N upgrades available." count.
///
/// Only ever fires on a single-field line, so a package name that begins with a
/// digit is not affected — the count lines carry no id column to parse.
fn is_count_line(line: &str) -> bool {
    let mut chars = line.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_digit())
        && matches!(chars.next(), Some(' '))
        && split_columns(line).len() < 4
}

/// Split a padded table line on runs of two or more spaces.
fn split_columns(line: &str) -> Vec<&str> {
    let bytes = line.as_bytes();
    let mut fields = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] != b' ' {
            i += 1;
            continue;
        }

        let run_start = i;
        while i < bytes.len() && bytes[i] == b' ' {
            i += 1;
        }
        if i - run_start >= 2 {
            if start < run_start {
                fields.push(&line[start..run_start]);
            }
            start = i;
        }
    }

    let tail = &line[start..];
    if !tail.trim().is_empty() {
        fields.push(tail);
    }
    fields
}

/// Upgrade one package by id.
pub fn update_package(package_id: &str, dry_run: bool) -> Result<String, String> {
    let id = package_id.trim();
    if id.is_empty() {
        return Err("No Winget package id was given, so nothing could be updated.".to_string());
    }

    if dry_run {
        return Ok(format!(
            "Would run: winget upgrade --id {} --silent (no change made).",
            id
        ));
    }

    run_upgrade(&["upgrade", "--id", id, "--silent"], id)
}

/// Upgrade every package winget reports as upgradable.
pub fn update_all(dry_run: bool) -> Result<String, String> {
    if dry_run {
        return Ok("Would run: winget upgrade --all --silent (no change made).".to_string());
    }

    run_upgrade(&["upgrade", "--all", "--silent"], "all upgradable packages")
}

/// Run a winget upgrade command and turn a non-zero exit into an error that
/// carries winget's own last output line.
fn run_upgrade(base_args: &[&str], target: &str) -> Result<String, String> {
    let mut args: Vec<&str> = base_args.to_vec();
    args.push("--accept-package-agreements");
    args.extend_from_slice(&COMMON_ARGS);

    let out = proc::run("winget.exe", &args)?;
    if !out.success {
        let detail = out.last_line();
        return Err(format!(
            "winget upgrade failed for {} (exit {}): {}",
            target,
            out.exit_code
                .map(|c| c.to_string())
                .unwrap_or_else(|| "no code".to_string()),
            if detail.is_empty() {
                "no output".to_string()
            } else {
                detail
            }
        ));
    }

    log_info("apps", &format!("Winget updated {}", target));
    let detail = out.last_line();
    if detail.is_empty() {
        Ok(format!("Winget finished updating {}.", target))
    } else {
        Ok(format!("Winget finished updating {}: {}", target, detail))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Captured from a real `winget upgrade --include-unknown` run, including
    /// the warning lines winget prints before the table.
    const SAMPLE: &str = r#"Failed in attempting to update the source: winget
A newer version of winget is available. Please update to the latest version.
Name                                   Id                          Version      Available    Source
------------------------------------------------------------------------------------------------------
7-Zip                                  7zip.7zip                   23.01        24.09        winget
Git                                    Git.Git                     2.43.0       2.44.0       winget
Microsoft Visual Studio Code           Microsoft.VisualStudioCode  1.86.0       1.87.0       winget
Notepad++ (64-bit x64)                 Notepad++.Notepad++         8.6.2        8.6.4        winget
2 upgrades available.
"#;

    #[test]
    fn upgrade_table_skips_banners_header_and_summary() {
        let updates = parse_upgrade_table(SAMPLE);
        assert_eq!(updates.len(), 4);
        assert_eq!(updates[0].package_id, "7zip.7zip");
        assert_eq!(updates[1].package_id, "Git.Git");
        assert_eq!(updates[1].name, "Git");
        assert_eq!(updates[1].current_version, "2.43.0");
        assert_eq!(updates[1].available_version, "2.44.0");
        assert_eq!(updates[1].state, UpdateState::UpdateAvailable);
        assert!(updates[1].app_name.is_none());
        assert!(updates[1].is_actionable());
    }

    #[test]
    fn upgrade_table_keeps_names_containing_spaces() {
        let updates = parse_upgrade_table(SAMPLE);
        assert_eq!(updates[2].name, "Microsoft Visual Studio Code");
        assert_eq!(updates[2].package_id, "Microsoft.VisualStudioCode");
        assert_eq!(updates[3].name, "Notepad++ (64-bit x64)");
        assert_eq!(updates[3].package_id, "Notepad++.Notepad++");
        assert_eq!(updates[3].available_version, "8.6.4");
    }

    #[test]
    fn digit_leading_package_names_are_not_treated_as_summary_lines() {
        let updates = parse_upgrade_table(SAMPLE);
        assert_eq!(updates[0].name, "7-Zip");
        assert_eq!(updates[0].available_version, "24.09");
        assert!(!is_count_line("7-Zip   7zip.7zip   23.01   24.09   winget"));
        assert!(is_count_line("2 upgrades available."));
    }

    #[test]
    fn summary_line_is_rejected_by_the_four_field_rule() {
        assert!(parse_upgrade_table("2 upgrades available.").is_empty());
        assert!(parse_upgrade_table("3 upgrades available.").is_empty());
    }

    #[test]
    fn upgrade_table_ignores_unparsable_lines() {
        let noise = "No installed package found matching input criteria.\n\
                     \n\
                     The following packages have an upgrade available, but require explicit targeting.\n\
                     Name   Id\n\
                     onlyonename\n";
        assert!(parse_upgrade_table(noise).is_empty());
        assert!(parse_upgrade_table("").is_empty());
    }

    #[test]
    fn header_recognition_is_case_insensitive() {
        assert!(is_header_or_separator(
            "NAME  ID  VERSION  AVAILABLE  SOURCE"
        ));
        assert!(is_header_or_separator("---  ----"));
        assert!(!is_header_or_separator(
            "Git   Git.Git   2.43.0   2.44.0   winget"
        ));
    }
}
