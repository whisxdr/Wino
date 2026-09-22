//! v2.6 CLI subcommand handlers.
//!
//! Split out of `commands.rs` so the argument definitions stay in one file and
//! the execution logic stays in another. Every handler prints a stable,
//! greppable shape: a header line, then one record per line prefixed with
//! `  - `. Handlers that accept `--json` emit a single JSON document on stdout
//! so the output can be piped into another tool.
//!
//! Exit codes are meaningful: a partial profile application, an unreachable
//! host, a failed DNS lookup, and a critical health verdict all exit non-zero so
//! a caller can branch on the result instead of parsing prose.

use crate::cli::commands::{
    AppsCommands, FeaturesCommands, HealthCenterCommands, NetworkCommands, PowerCommands,
    ProfileCommands, SecurityCenterCommands, StorageCommands,
};
use crate::core::i18n::{tr, Lang};

/// Translate a dictionary key for CLI output.
///
/// CLI output goes to a person reading a terminal, so the keys the GUI resolves
/// through `tr` are resolved here too. English is used unconditionally: a
/// headless script's output should not change meaning based on a UI preference.
fn t(key: &str) -> &'static str {
    tr(Lang::En, key)
}

pub fn run_apps_cli(action: Option<AppsCommands>) {
    use crate::apps::{manager, uninstall, winget};

    match action.unwrap_or(AppsCommands::List { json: false }) {
        AppsCommands::List { json } => {
            let apps = crate::apps::scanner::scan_installed_apps();
            if json {
                match serde_json::to_string_pretty(&apps) {
                    Ok(text) => println!("{}", text),
                    Err(e) => eprintln!("[ERROR] Failed to serialize applications: {}", e),
                }
                return;
            }
            println!("Installed applications ({} total):", apps.len());
            for app in &apps {
                let size = {
                    let s = app.size_label();
                    if s.is_empty() {
                        "-".to_string()
                    } else {
                        s
                    }
                };
                let uninstall = if app.is_uninstallable() {
                    app.uninstall
                        .as_ref()
                        .map(|u| u.kind_label())
                        .unwrap_or("-")
                } else {
                    "unavailable"
                };
                println!(
                    "  - [{}] {} | {} | {} | {} | uninstall={}",
                    app.source.label(),
                    app.display_name,
                    if app.publisher.is_empty() {
                        "unknown publisher"
                    } else {
                        &app.publisher
                    },
                    app.version,
                    size,
                    uninstall
                );
            }
        }
        AppsCommands::Updates { json } => {
            let status = winget::winget_status();
            if !status.available {
                eprintln!("[ERROR] winget not found. {}", status.detail);
                std::process::exit(1);
            }
            let updates = winget::check_updates();
            if json {
                match serde_json::to_string_pretty(&updates) {
                    Ok(text) => println!("{}", text),
                    Err(e) => eprintln!("[ERROR] Failed to serialize updates: {}", e),
                }
                return;
            }
            if updates.is_empty() {
                println!("No package updates reported by Winget.");
                return;
            }
            println!("{} package update(s) available:", updates.len());
            for u in &updates {
                println!(
                    "  - {} [{}] {} -> {}",
                    u.name, u.package_id, u.current_version, u.available_version
                );
            }
        }
        AppsCommands::Update { id, all, dry_run } => {
            let result = match (id, all) {
                (Some(package_id), _) => winget::update_package(&package_id, dry_run),
                (None, true) => winget::update_all(dry_run),
                (None, false) => {
                    eprintln!("Specify --id <package> or --all.");
                    std::process::exit(2);
                }
            };
            match result {
                Ok(msg) => println!("[OK] {}", msg),
                Err(e) => {
                    eprintln!("[ERROR] {}", e);
                    std::process::exit(1);
                }
            }
        }
        AppsCommands::Uninstall { name, dry_run } => {
            let apps = crate::apps::scanner::scan_installed_apps();
            let matches = manager::filter_apps(&apps, &name.to_lowercase(), None);
            match matches.len() {
                0 => {
                    eprintln!("No installed application matches '{}'.", name);
                    std::process::exit(1);
                }
                1 => {
                    let record = matches[0];
                    match uninstall::uninstall_app(record, dry_run) {
                        Ok(msg) => println!("[OK] {}", msg),
                        Err(e) => {
                            eprintln!("[ERROR] {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                _ => {
                    eprintln!(
                        "'{}' matches {} applications. Be more specific:",
                        name,
                        matches.len()
                    );
                    for app in matches.iter().take(20) {
                        eprintln!("  - {} [{}]", app.display_name, app.source.label());
                    }
                    std::process::exit(2);
                }
            }
        }
    }
}

pub fn run_profile_cli(action: Option<ProfileCommands>) {
    use crate::profiles::manager;

    match action.unwrap_or(ProfileCommands::List) {
        ProfileCommands::List => {
            let profiles = manager::load_profiles();
            println!("Profiles ({} total):", profiles.len());
            for p in &profiles {
                println!(
                    "  - [{}] {} | {} step(s) | max risk {} | {}",
                    if p.is_builtin() { "built-in" } else { "user" },
                    p.name,
                    p.steps.len(),
                    p.max_risk().as_str(),
                    p.description
                );
            }
        }
        ProfileCommands::Show { profile } => {
            let profiles = manager::load_profiles();
            let Some(found) = find_profile(&profiles, &profile) else {
                eprintln!("Profile '{}' not found. Run `wino profile list`.", profile);
                std::process::exit(1);
            };
            println!("Profile: {} ({})", found.name, found.id);
            println!("{}", found.description);
            println!();
            println!("Steps ({}):", found.steps.len());
            for desc in manager::preview_profile(found) {
                println!(
                    "  - {} | risk {} | reversible={} | admin={} | reboot={}",
                    desc.name,
                    desc.risk.as_str(),
                    desc.reversible,
                    desc.requires_admin,
                    desc.requires_reboot
                );
                if !desc.reason.is_empty() {
                    println!("      {}", desc.reason);
                }
            }
            let blocked = found.blocked_steps();
            if !blocked.is_empty() {
                println!();
                println!(
                    "{} step(s) would be blocked by the Safety Engine.",
                    blocked.len()
                );
            }
        }
        ProfileCommands::Apply { profile, dry_run } => {
            let profiles = manager::load_profiles();
            let Some(found) = find_profile(&profiles, &profile) else {
                eprintln!("Profile '{}' not found. Run `wino profile list`.", profile);
                std::process::exit(1);
            };
            let report = manager::apply_profile(found, dry_run);
            for outcome in &report.outcomes {
                let marker = match (&outcome.success, &outcome.skipped_reason) {
                    (true, _) => "[OK]",
                    (false, Some(_)) => "[SKIP]",
                    (false, None) => "[FAIL]",
                };
                println!("  {} {}", marker, outcome.label);
                if let Some(reason) = &outcome.skipped_reason {
                    println!("      {}", reason);
                }
            }
            println!("{}", report.summary());
            // A partial application must not exit as success.
            if report.is_partial() {
                std::process::exit(1);
            }
        }
        ProfileCommands::Export {
            profile,
            destination,
        } => {
            let profiles = manager::load_profiles();
            let Some(found) = find_profile(&profiles, &profile) else {
                eprintln!("Profile '{}' not found.", profile);
                std::process::exit(1);
            };
            match manager::export_profile(found, &destination) {
                Ok(msg) => println!("[OK] {}", msg),
                Err(e) => {
                    eprintln!("[ERROR] {}", e);
                    std::process::exit(1);
                }
            }
        }
        ProfileCommands::Import { path } => match manager::import_profile(&path) {
            Ok(profile) => match manager::save_user_profile(&profile) {
                Ok(()) => println!("[OK] Imported profile '{}'.", profile.name),
                Err(e) => {
                    eprintln!("[ERROR] {}", e);
                    std::process::exit(1);
                }
            },
            Err(e) => {
                eprintln!("[ERROR] {}", e);
                std::process::exit(1);
            }
        },
    }
}

/// Resolve a profile by id or by case-insensitive name.
fn find_profile<'a>(
    profiles: &'a [crate::profiles::models::Profile],
    needle: &str,
) -> Option<&'a crate::profiles::models::Profile> {
    profiles
        .iter()
        .find(|p| p.id.eq_ignore_ascii_case(needle))
        .or_else(|| {
            profiles
                .iter()
                .find(|p| p.name.eq_ignore_ascii_case(needle))
        })
}

pub fn run_power_cli(action: Option<PowerCommands>) {
    use crate::power::{manager, settings};

    match action.unwrap_or(PowerCommands::Plans) {
        PowerCommands::Plans => {
            let plans = manager::list_plans();
            let active = manager::active_plan_guid().unwrap_or_default();
            println!("Power plans ({} found):", plans.len());
            for p in &plans {
                println!(
                    "  - {}{} | {} | {}",
                    p.name,
                    if p.guid.eq_ignore_ascii_case(&active) {
                        " (active)"
                    } else {
                        ""
                    },
                    p.guid,
                    t(p.kind_i18n_key())
                );
            }
        }
        PowerCommands::Settings => {
            let values = settings::read_all_settings();
            println!("Advanced power settings ({} reported):", values.len());
            for s in &values {
                if s.is_available() {
                    println!(
                        "  - {} | AC {} | DC {}",
                        s.name,
                        s.format_value(s.ac_value),
                        s.format_value(s.dc_value)
                    );
                } else {
                    println!("  - {} | not exposed by the active scheme", s.name);
                }
            }
        }
        PowerCommands::Set { plan } => match manager::set_active_plan(&plan, false) {
            Ok(msg) => println!("[OK] {}", msg),
            Err(e) => {
                eprintln!("[ERROR] {}", e);
                std::process::exit(1);
            }
        },
        PowerCommands::CreateUltimate => match manager::ensure_ultimate_performance() {
            Ok(msg) => println!("[OK] {}", msg),
            Err(e) => {
                eprintln!("[ERROR] {}", e);
                std::process::exit(1);
            }
        },
    }
}

pub fn run_features_cli(action: Option<FeaturesCommands>) {
    use crate::windows_features::{manager, scanner};

    match action.unwrap_or(FeaturesCommands::List { json: false }) {
        FeaturesCommands::List { json } => {
            let features = scanner::scan_features();
            if json {
                match serde_json::to_string_pretty(&features) {
                    Ok(text) => println!("{}", text),
                    Err(e) => eprintln!("[ERROR] Failed to serialize features: {}", e),
                }
                return;
            }
            if features.is_empty() {
                println!(
                    "No optional features were reported. DISM requires administrator privileges to enumerate them."
                );
                return;
            }
            println!("Optional features ({} reported):", features.len());
            for f in &features {
                println!(
                    "  - {} | {} | risk {} | {}",
                    f.name,
                    f.state.label(),
                    f.risk.as_str(),
                    f.display_name
                );
            }
        }
        FeaturesCommands::Set {
            name,
            enable,
            dry_run,
        } => match manager::set_feature_enabled(&name, enable, dry_run) {
            Ok(msg) => println!("[OK] {}", msg),
            Err(e) => {
                eprintln!("[ERROR] {}", e);
                std::process::exit(1);
            }
        },
    }
}

pub fn run_network_cli(action: Option<NetworkCommands>) {
    use crate::network::diagnostics;

    match action.unwrap_or(NetworkCommands::Status { json: false }) {
        NetworkCommands::Status { json } => {
            let report = diagnostics::run_network_scan();
            if json {
                match serde_json::to_string_pretty(&report) {
                    Ok(text) => println!("{}", text),
                    Err(e) => eprintln!("[ERROR] Failed to serialize network report: {}", e),
                }
                return;
            }
            println!("Adapters ({} detected):", report.adapters.len());
            for a in &report.adapters {
                println!(
                    "  - {}{} | {} | ipv4=[{}] | gw=[{}] | dns=[{}]",
                    a.friendly_name,
                    if a.friendly_name == report.active_adapter {
                        " (active)"
                    } else {
                        ""
                    },
                    if a.is_up { "up" } else { "down" },
                    a.ipv4
                        .iter()
                        .map(|x| x.address.clone())
                        .collect::<Vec<_>>()
                        .join(", "),
                    a.gateways.join(", "),
                    a.dns_servers.join(", ")
                );
            }
            if report.active_adapter.is_empty() {
                println!("No active adapter detected.");
            }
        }
        NetworkCommands::Gateway => {
            let report = diagnostics::run_network_scan();
            match diagnostics::test_gateway(&report, 4) {
                Ok(r) => {
                    println!("{}", r.summary());
                    if !r.is_reachable() {
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("[ERROR] {}", e);
                    std::process::exit(1);
                }
            }
        }
        NetworkCommands::Ping { host, count } => match diagnostics::ping_host(&host, count, 1500) {
            Ok(r) => {
                println!("{}", r.summary());
                if !r.is_reachable() {
                    std::process::exit(1);
                }
            }
            Err(e) => {
                eprintln!("[ERROR] {}", e);
                std::process::exit(1);
            }
        },
        NetworkCommands::Lookup { host } => {
            let r = diagnostics::dns_lookup(&host);
            println!("{}", r.summary());
            for a in &r.addresses {
                println!("  - {}", a);
            }
            if !r.succeeded() {
                std::process::exit(1);
            }
        }
        NetworkCommands::Latency { host, count } => {
            match diagnostics::measure_latency(&host, count) {
                Ok(r) => match r.avg_rtt_ms() {
                    Some(avg) => println!(
                        "{}: {:.1} ms average over {} replies",
                        r.resolved_address, avg, r.received
                    ),
                    None => {
                        println!("No reply from {}.", r.host);
                        std::process::exit(1);
                    }
                },
                Err(e) => {
                    eprintln!("[ERROR] {}", e);
                    std::process::exit(1);
                }
            }
        }
        NetworkCommands::Loss { host, count } => {
            match diagnostics::measure_packet_loss(&host, count) {
                Ok(r) => {
                    println!("{}", r.summary());
                    if r.received < r.sent {
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("[ERROR] {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}

pub fn run_health_center_cli(action: Option<HealthCenterCommands>) {
    use crate::health::{center, integrity};

    match action.unwrap_or(HealthCenterCommands::Scan {
        json: false,
        dism: false,
    }) {
        HealthCenterCommands::Scan { json, dism } => {
            let report = center::run_health_scan(dism);
            if json {
                match serde_json::to_string_pretty(&report) {
                    Ok(text) => println!("{}", text),
                    Err(e) => eprintln!("[ERROR] Failed to serialize health report: {}", e),
                }
                return;
            }
            println!("Overall: {}", report.overall.as_str());
            println!("Checks ({}):", report.checks.len());
            for c in &report.checks {
                println!(
                    "  - {} | {} | {}",
                    t(&c.name_key),
                    c.state.as_str(),
                    c.detail
                );
                if !c.recommendation.is_empty() {
                    println!("      {}", c.recommendation);
                }
            }
            if report.has_unknowns() {
                println!();
                println!(
                    "Some checks could not be queried. Wino reports Unknown rather than guessing."
                );
            }
            if report.overall == center::HealthState::Critical {
                std::process::exit(1);
            }
        }
        HealthCenterCommands::Sfc => match integrity::run_sfc_scan() {
            Ok(out) => {
                println!("{}", out);
                println!("Verdict: {:?}", integrity::interpret_sfc_output(&out));
            }
            Err(e) => {
                eprintln!("[ERROR] {}", e);
                std::process::exit(1);
            }
        },
        HealthCenterCommands::DismCheck => {
            match integrity::run_dism(integrity::DismMode::CheckHealth) {
                Ok(out) => println!("{}", out),
                Err(e) => {
                    eprintln!("[ERROR] {}", e);
                    std::process::exit(1);
                }
            }
        }
        HealthCenterCommands::DismScan => {
            match integrity::run_dism(integrity::DismMode::ScanHealth) {
                Ok(out) => println!("{}", out),
                Err(e) => {
                    eprintln!("[ERROR] {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}

pub fn run_storage_cli(action: Option<StorageCommands>) {
    use crate::storage::{analyzer, scanner};

    match action.unwrap_or(StorageCommands::Scan {
        json: false,
        summary: false,
    }) {
        StorageCommands::Scan { json, summary } => {
            let tree = scanner::scan_system_drive();
            if json {
                match serde_json::to_string_pretty(&tree) {
                    Ok(text) => println!("{}", text),
                    Err(e) => eprintln!("[ERROR] Failed to serialize storage tree: {}", e),
                }
                return;
            }

            println!(
                "System drive: {} used of {} ({} free)",
                analyzer::format_bytes(tree.used_bytes),
                analyzer::format_bytes(tree.total_bytes),
                analyzer::format_bytes(tree.free_bytes)
            );
            println!(
                "Scanned {} files in {} directories.",
                tree.files_scanned, tree.dirs_scanned
            );
            if tree.skipped_inaccessible > 0 || tree.skipped_reparse > 0 {
                println!(
                    "Skipped {} inaccessible path(s) and {} reparse point(s).",
                    tree.skipped_inaccessible, tree.skipped_reparse
                );
            }
            if tree.limit_reached {
                println!("Scan stopped at the configured entry limit. Results cover the portion scanned.");
            }

            println!();
            println!("Usage by category:");
            for (bucket, bytes, pct) in analyzer::bucket_percentages(&tree) {
                println!(
                    "  - {:<14} {:>12}  {:>5.1}%",
                    bucket.label(),
                    analyzer::format_bytes(bytes),
                    pct
                );
            }

            if !summary {
                println!();
                println!("Largest directories:");
                for entry in analyzer::top_entries(&tree, 20) {
                    println!(
                        "  - {:<40} {:>12}  {} file(s)",
                        entry.name,
                        analyzer::format_bytes(entry.size_bytes),
                        entry.file_count
                    );
                }
            }

            println!();
            println!("The analyzer deletes nothing. Use `wino cleanup` to reclaim space.");
        }
        StorageCommands::LargeFiles { limit } => {
            let tree = scanner::scan_system_drive();
            let files = scanner::largest_files_in(&tree, limit);
            if files.is_empty() {
                println!("No files above the configured size threshold.");
                return;
            }
            println!("Largest files ({}):", files.len());
            for f in &files {
                let age = f
                    .age_days
                    .map(|d| format!("{}d", d))
                    .unwrap_or_else(|| "unknown".to_string());
                println!(
                    "  - {:>12}  {:>8}  {}",
                    analyzer::format_bytes(f.size_bytes),
                    age,
                    f.path
                );
            }
        }
    }
}

pub fn run_security_center_cli(action: Option<SecurityCenterCommands>) {
    use crate::security::center;

    match action.unwrap_or(SecurityCenterCommands::Scan { json: false }) {
        SecurityCenterCommands::Scan { json } => {
            let report = center::scan_security_center();
            if json {
                match serde_json::to_string_pretty(&report) {
                    Ok(text) => println!("{}", text),
                    Err(e) => eprintln!("[ERROR] Failed to serialize security report: {}", e),
                }
                return;
            }
            println!("Security center:");
            for item in &report.items {
                println!(
                    "  - {} | {} | {}",
                    t(&item.name_key),
                    match item.state {
                        center::ProtectionState::On => "On",
                        center::ProtectionState::Off => "Off",
                        center::ProtectionState::Warning => "Warning",
                        center::ProtectionState::Unknown => "Unknown",
                    },
                    item.detail
                );
                if !item.extra.is_empty() {
                    println!("      {}", item.extra);
                }
            }
            println!();
            println!("Wino reports protection state only. It never disables a security component.");
            if !report.warnings().is_empty() {
                std::process::exit(1);
            }
        }
    }
}

pub fn run_recommend_cli() {
    let items = crate::recommendations::analyzer::analyze_system();
    if items.is_empty() {
        println!("No recommendations. Nothing measured needs attention.");
        return;
    }
    println!("{} recommendation(s) from measured state:", items.len());
    for r in &items {
        println!(
            "  - [{}] [{}] {} ({})",
            t(r.severity.i18n_key()),
            t(r.area.i18n_key()),
            r.title,
            r.measured
        );
    }
    println!();
    println!("Every recommendation is an observation. Run the matching command to review and apply a change.");
}
