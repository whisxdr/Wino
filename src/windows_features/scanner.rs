//! Optional-feature enumeration through `dism.exe`.
//!
//! DISM is used rather than a native API because the crate exposes no DISM
//! surface: the `windows` crate's `Win32_System_ApplicationInstallationAndServicing`
//! feature is the **MSI** surface (`MsiEnumProducts` and friends), which reports
//! installed products, not optional Windows components. `dism.exe` is the
//! documented interface for optional components and is present on every
//! supported Windows install.
//!
//! Two constraints shape the scan:
//!
//! * **The table format is parsed, not scraped for keywords.** `dism /get-features
//!   /format:table` is a fixed two-column table; a row is accepted only when it
//!   has exactly two columns, so prose lines, the header, the separator, and the
//!   `The operation completed successfully.` trailer are dropped structurally
//!   rather than by string matching alone.
//! * **Per-feature queries are budgeted.** Dependencies are not in the table
//!   output, so they need one `dism /get-featureinfo` call each. A stock install
//!   reports roughly 200 features and each call costs 0.5-2 seconds, so
//!   dependencies are read only for curated features and only up to
//!   [`MAX_DEPENDENCY_QUERIES`] calls. Everything else reports an empty
//!   dependency list, which the model treats as "no known dependencies" rather
//!   than "none exist".

use crate::core::logger::{log_info, log_warn};
use crate::core::proc;
use crate::core::safety::RiskLevel;
use crate::windows_features::models::{
    apply_curation, FeatureState, WindowsFeature, CURATED_FEATURES,
};

/// Upper bound on `dism /get-featureinfo` invocations per scan.
///
/// Each call is a process spawn plus a COM/DISM service round trip, measured in
/// hundreds of milliseconds to a few seconds. Forty calls keeps a full scan
/// under about a minute on a slow machine while still covering every curated
/// feature; a machine with more curated features than this reports the
/// remaining ones without dependencies.
const MAX_DEPENDENCY_QUERIES: usize = 40;

/// Enumerate the machine's optional features with Wino's curation applied.
///
/// Returns an empty list when DISM cannot be run: the UI then shows "no
/// features reported", which is a truthful statement about what was read. A
/// failed scan must never be presented as "no features installed".
pub fn scan_features() -> Vec<WindowsFeature> {
    let args = ["/online", "/get-features", "/format:table"];
    let output = match proc::run("dism.exe", &args) {
        Ok(output) => output,
        Err(error) => {
            log_warn("features", &format!("Feature scan failed: {}", error));
            return Vec::new();
        }
    };

    if !output.success {
        log_warn(
            "features",
            &format!(
                "Feature scan failed (dism.exe exited with {}): {}",
                output
                    .exit_code
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "no code".to_string()),
                output.last_line()
            ),
        );
        return Vec::new();
    }

    let mut features = parse_dism_feature_table(&output.stdout);
    for feature in &mut features {
        // The table has no friendly-name column, so the raw name is the only
        // display name available; curation replaces it for known features.
        let (display_name, risk, impact, curated) = apply_curation(&feature.name, "");
        feature.display_name = display_name;
        feature.risk = risk;
        feature.impact = impact;
        feature.curated = curated;
    }

    let mut queries = 0usize;
    for feature in features.iter_mut().filter(|f| f.curated) {
        if queries >= MAX_DEPENDENCY_QUERIES {
            log_warn(
                "features",
                &format!(
                    "Dependency lookup stopped after {} queries; remaining curated features are listed without dependencies",
                    MAX_DEPENDENCY_QUERIES
                ),
            );
            break;
        }
        queries += 1;

        let arg = format!("/featurename:{}", feature.name);
        let args = ["/online", "/get-featureinfo", arg.as_str()];
        match proc::run("dism.exe", &args) {
            Ok(output) if output.success => {
                feature.dependencies = parse_feature_dependencies(&output.stdout);
            }
            Ok(output) => log_warn(
                "features",
                &format!(
                    "Dependency lookup for {} failed: {}",
                    feature.name,
                    output.last_line()
                ),
            ),
            Err(error) => log_warn(
                "features",
                &format!("Dependency lookup for {} failed: {}", feature.name, error),
            ),
        }
    }

    sort_features(&mut features);
    log_info(
        "features",
        &format!(
            "Feature scan complete: {} features, {} curated, {} dependency queries",
            features.len(),
            features.iter().filter(|f| f.curated).count(),
            queries
        ),
    );
    features
}

/// Parse the `dism /online /get-features /format:table` output.
///
/// Pure, so the table shape is testable against a captured sample. Rows are
/// accepted only when they are two columns wide; the state is mapped through
/// [`FeatureState::parse`], which reports [`FeatureState::Unknown`] for a value
/// it does not recognize instead of assuming a state.
pub fn parse_dism_feature_table(output: &str) -> Vec<WindowsFeature> {
    let lines: Vec<&str> = output.lines().map(str::trim).collect();
    let mut features = Vec::new();
    let mut table_started = false;

    for (index, line) in lines.iter().enumerate() {
        if line.is_empty() {
            continue;
        }
        if is_separator(line) {
            table_started = true;
            continue;
        } // The header is recognized independently of the borders, so a build
          // that prints the table without its rules still parses.
        if is_header_row(&lines, index) {
            table_started = true;
            continue;
        }
        // Prose before the table: the tool banner, image version, and the
        // "Features listing for package" line.
        if !table_started {
            continue;
        }

        let columns = split_columns(line);
        // A two-column row is the only shape the table produces. Anything else
        // is a banner or trailer line whose split would be a guess.
        if columns.len() != 2 {
            continue;
        }

        let name = columns[0];
        let state_text = columns[1];
        if name.is_empty() || state_text.is_empty() {
            continue;
        }
        // Trailer: `The operation completed successfully.` (and its localized
        // equivalents, which never sit in a two-column row).
        if name.to_lowercase().contains("operation completed") {
            continue;
        }

        features.push(WindowsFeature {
            name: name.to_string(),
            display_name: name.to_string(),
            state: FeatureState::parse(state_text),
            dependencies: Vec::new(),
            risk: RiskLevel::Low,
            impact: String::new(),
            curated: false,
        });
    }

    features
}

/// Parse the `Depends On` section of `dism /online /get-featureinfo` output.
///
/// The section is a label line followed by one feature name per line; some
/// builds emit the names comma-separated on the label line itself, so both
/// shapes are read. The section ends at the next field label (`State : ...`),
/// at a blank line, or at end of output.
pub fn parse_feature_dependencies(output: &str) -> Vec<String> {
    let mut dependencies: Vec<String> = Vec::new();
    let mut in_section = false;

    for raw in output.lines() {
        let line = raw.trim();

        if line.is_empty() {
            in_section = false;
            continue;
        }

        if let Some(remainder) = dependency_label_remainder(line) {
            in_section = true;
            push_dependency(&mut dependencies, remainder);
            continue;
        }

        if is_field_label(line) {
            in_section = false;
            continue;
        }

        if in_section {
            push_dependency(&mut dependencies, line);
        }
    }

    dependencies
}

/// Curated features first, in [`CURATED_FEATURES`] order, then the rest
/// alphabetically. Pure, so the ordering rule is testable without DISM.
fn sort_features(features: &mut [WindowsFeature]) {
    features.sort_by(|a, b| {
        curation_rank(&a.name)
            .cmp(&curation_rank(&b.name))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
}

/// Position in the curated list, or `usize::MAX` for an uncurated feature so
/// curated entries always sort ahead of the alphabetized remainder.
fn curation_rank(name: &str) -> usize {
    CURATED_FEATURES
        .iter()
        .position(|curated| curated.name.eq_ignore_ascii_case(name))
        .unwrap_or(usize::MAX)
}

/// Split a table row into columns.
///
/// The table's column separator is a literal `|`, so that is preferred: DISM
/// pads the name column to a fixed width, but a name longer than that width
/// pushes the separator out with only one space of padding, which a
/// whitespace-only split would join into the state. Falls back to splitting on
/// runs of two or more spaces for a build that prints the table without bars —
/// feature names contain single spaces (`.NET Framework 3.5 (includes .NET 2.0
/// and 3.0)`), so a single space must never split a column.
fn split_columns(line: &str) -> Vec<&str> {
    if let Some((name, state)) = line.split_once('|') {
        return vec![name.trim(), state.trim()];
    }

    let mut columns = Vec::new();
    let mut field_start = 0usize;
    let mut chars = line.char_indices().peekable();

    while let Some((index, ch)) = chars.next() {
        if !ch.is_whitespace() {
            continue;
        }

        let run_start = index;
        let mut run_end = index + ch.len_utf8();
        while let Some(&(next_index, next_ch)) = chars.peek() {
            if !next_ch.is_whitespace() {
                break;
            }
            chars.next();
            run_end = next_index + next_ch.len_utf8();
        }

        if run_end - run_start >= 2 {
            columns.push(line[field_start..run_start].trim());
            field_start = run_end;
        }
    }

    columns.push(line[field_start..].trim());
    columns
}

/// Table border: dashes, equals signs, pluses, bars, and the spaces DISM puts
/// between the border's column groups — and nothing else.
///
/// `dism /format:table` prints its rule as `---- | ----`, so a border contains
/// spaces. Testing for "only bar-like characters" would miss it, and the rule
/// would then be split on its bar and read as a two-column feature row.
fn is_separator(line: &str) -> bool {
    !line.is_empty()
        && line
            .chars()
            .all(|ch| matches!(ch, '-' | '=' | '+' | '|' | ' ' | '\t'))
        // A border must contain a rule character; a blank line is not a border.
        && line.chars().any(|ch| matches!(ch, '-' | '=' | '+' | '|'))
}

/// The header row sits between the two border lines; the literal `Feature Name`
/// / `State` check covers the English output directly.
fn is_header_row(lines: &[&str], index: usize) -> bool {
    let name_column = split_columns(lines[index])
        .first()
        .copied()
        .unwrap_or_default();

    if name_column.eq_ignore_ascii_case("feature name") {
        return true;
    }

    let previous_is_border = index > 0 && is_separator(lines[index - 1]);
    let next_is_border = lines.get(index + 1).is_some_and(|line| is_separator(line));
    previous_is_border && next_is_border
}

/// When `line` opens the dependency section, the text after its `:`.
fn dependency_label_remainder(line: &str) -> Option<&str> {
    let (label, remainder) = match line.split_once(':') {
        Some((label, remainder)) => (label, Some(remainder)),
        None => (line, None),
    };

    let label = label.trim();
    let is_dependency_label =
        label.eq_ignore_ascii_case("depends on") || label.eq_ignore_ascii_case("dependencies");
    if !is_dependency_label {
        return None;
    }

    Some(remainder.unwrap_or("").trim())
}

/// `Feature Name : X`, `State : Disabled` — any other field of the same block.
fn is_field_label(line: &str) -> bool {
    line.ends_with(':') || line.contains(" : ")
}

/// Accept a dependency candidate: comma-separated lists are split, and anything
/// that is not a canonical feature name (`None`, prose, blank) is dropped.
fn push_dependency(dependencies: &mut Vec<String>, candidate: &str) {
    for part in candidate.split(',') {
        let name = part.trim();
        if name.is_empty()
            || name.eq_ignore_ascii_case("none")
            || name.eq_ignore_ascii_case("(none)")
            || name.contains(char::is_whitespace)
        {
            continue;
        }
        if !dependencies.iter().any(|existing| existing == name) {
            dependencies.push(name.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::safety::RiskLevel;
    use crate::windows_features::models::find_curated;

    /// Captured `dism.exe /online /get-features /format:table` output shape,
    /// including the banner, both borders, the header, and the trailer.
    const FEATURE_TABLE: &str = "\
Deployment Image Servicing and Management tool
Version: 10.0.22621.2283

Image Version: 10.0.22621.2283

Features listing for package : Microsoft-Windows-Foundation-Package~31bf3856ad364e35~amd64~~10.0.22621.1

------------------------------------------------------------------------------------
Feature Name                                     | State
------------------------------------------------------------------------------------
.NET Framework 3.5 (includes .NET 2.0 and 3.0)   | Disabled
Microsoft-Hyper-V-All                            | Disabled
Microsoft-Windows-Subsystem-Linux                | Enabled
MicrosoftWindowsPowerShellV2Root                 | Disabled
OpenSSH.Server                                   | Enabled
TelnetClient                                     | EnablePending
VirtualMachinePlatform                           | Disabled
Weird-State-Feature                              | SomethingElse

The operation completed successfully.
";

    #[test]
    fn table_parsing_skips_banner_header_border_and_trailer() {
        let features = parse_dism_feature_table(FEATURE_TABLE);

        assert_eq!(features.len(), 8);
        assert_eq!(
            features[0].name,
            ".NET Framework 3.5 (includes .NET 2.0 and 3.0)"
        );
        assert_eq!(features[0].state, FeatureState::Disabled);
        assert_eq!(features[1].name, "Microsoft-Hyper-V-All");
        assert_eq!(features[5].name, "TelnetClient");
        assert_eq!(features[5].state, FeatureState::RequiresReboot);
        // Unrecognized state is reported as Unknown, never guessed at.
        assert_eq!(features[7].state, FeatureState::Unknown);

        // No banner, header, border, or trailer leaked into the list.
        for feature in &features {
            assert!(!feature.name.to_lowercase().contains("deployment image"));
            assert!(!feature.name.eq_ignore_ascii_case("feature name"));
            assert!(!feature.name.to_lowercase().contains("operation completed"));
            assert!(!feature.name.starts_with('-'));
        }
    }

    #[test]
    fn table_parsing_tolerates_empty_and_border_only_output() {
        assert!(parse_dism_feature_table("").is_empty());
        assert!(parse_dism_feature_table("The operation completed successfully.\n").is_empty());
        assert!(parse_dism_feature_table("----------\n").is_empty());
    }

    #[test]
    fn column_split_keeps_single_spaces_inside_a_name() {
        let columns = split_columns(".NET Framework 3.5 (includes .NET 2.0 and 3.0)   | Disabled");
        assert_eq!(columns.len(), 2);
        assert_eq!(columns[0], ".NET Framework 3.5 (includes .NET 2.0 and 3.0)");
        assert_eq!(columns[1].trim().trim_start_matches('|').trim(), "Disabled");
    }

    #[test]
    fn dependency_parsing_reads_both_shapes_and_stops_at_the_next_field() {
        let one_per_line = "\
Feature Name : Containers-DisposableClientVM
Display Name : Windows Sandbox
State : Disabled

Depends On : 
Containers
Hyper-V
VirtualMachinePlatform
";
        assert_eq!(
            parse_feature_dependencies(one_per_line),
            vec![
                "Containers".to_string(),
                "Hyper-V".to_string(),
                "VirtualMachinePlatform".to_string()
            ]
        );

        let comma_separated = "State : Enabled\nDepends On : A,B , C\n";
        assert_eq!(
            parse_feature_dependencies(comma_separated),
            vec!["A".to_string(), "B".to_string(), "C".to_string()]
        );

        let none = "State : Enabled\nDepends On : None\n";
        assert!(parse_feature_dependencies(none).is_empty());

        // No dependency section at all, and a section that must not swallow the
        // following field labels.
        assert!(parse_feature_dependencies("Feature Name : NetFx3\nState : Disabled\n").is_empty());
    }

    #[test]
    fn dependencies_are_deduplicated_and_ordered() {
        let output = "Depends On :\nHyper-V\nHyper-V\nVirtualMachinePlatform\n";
        assert_eq!(
            parse_feature_dependencies(output),
            vec!["Hyper-V".to_string(), "VirtualMachinePlatform".to_string()]
        );
    }

    #[test]
    fn sorting_puts_curated_features_first_in_curated_order() {
        let feature = |name: &str| WindowsFeature {
            name: name.to_string(),
            display_name: name.to_string(),
            state: FeatureState::Disabled,
            dependencies: Vec::new(),
            risk: RiskLevel::Low,
            impact: String::new(),
            curated: find_curated(name).is_some(),
        };

        let mut features = vec![
            feature("Zebra-Feature"),
            feature("OpenSSH.Server"),
            feature("Alpha-Feature"),
            feature("Microsoft-Hyper-V-All"),
            feature("openssh.client"),
        ];
        sort_features(&mut features);

        let names: Vec<&str> = features.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                // CURATED_FEATURES order: Hyper-V before OpenSSH.Client before OpenSSH.Server.
                "Microsoft-Hyper-V-All",
                "openssh.client",
                "OpenSSH.Server",
                // Then the remainder, alphabetically and case-insensitively.
                "Alpha-Feature",
                "Zebra-Feature",
            ]
        );
        assert!(features[0].curated);
        assert!(!features[3].curated);
    }
}
