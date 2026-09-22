//! System analysis: measure once, then apply the pure threshold rules.
//!
//! This module is the only place where the recommendation engine touches the
//! machine. Every scanner is called exactly once per analysis, its numbers are
//! handed to [`crate::recommendations::rules`], and the resulting observations
//! are sorted and returned. Nothing here changes a setting: analysis is
//! read-only by construction, which is what lets the Recommendations view be
//! trusted as a report rather than a prompt to act.
//!
//! The winget update check is gated on [`AppConfig::apps`]`::scan_winget_metadata`
//! because it is the one measurement that spawns a subprocess and takes seconds.
//! With the setting off, the update count is *unknown* and no update
//! observation is emitted — an unknown value is never reported as zero.

use crate::apps::winget::{check_updates, winget_status};
use crate::core::config::AppConfig;
use crate::core::logger::log_info;
use crate::health::defender::check_security_health;
use crate::health::update::check_update_health;
use crate::memory::monitor::capture_memory_snapshot;
use crate::monitoring::disk::get_primary_disk_stats;
use crate::power::manager::list_plans;
use crate::power::models::{PowerPlan, PowerPlanKind};
use crate::privacy::scanner::scan_privacy_items;
use crate::recommendations::models::{sort_recommendations, Recommendation};
use crate::recommendations::rules;
use crate::services::scanner::scan_services;
use crate::startup::scanner::scan_startup_items;
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

/// Analyze the machine and return every observation the rules produced.
///
/// A scan that finds nothing worth reporting returns an empty list, which is a
/// valid and expected result.
pub fn analyze_system() -> Vec<Recommendation> {
    let config = AppConfig::load();
    let mut items = Vec::new();

    // ---- Memory ----
    let snapshot = capture_memory_snapshot();
    let ram = &snapshot.stats;
    items.extend(rules::ram_usage(
        ram.usage_pct,
        ram.used_bytes,
        ram.total_bytes,
    ));
    items.extend(rules::commit_charge(
        ram.commit_used_bytes,
        ram.commit_limit_bytes,
    ));

    // ---- Startup ----
    // Only entries that are actually enabled are counted: a disabled entry is
    // not launched at sign-in, so it cannot be part of a startup-load
    // observation. `enabled` is a real measurement — the scanner reads the
    // StartupApproved record Windows itself keeps.
    let startup_items = scan_startup_items();
    let startup_high = startup_items
        .iter()
        .filter(|item| item.enabled && item.impact == "High")
        .count();
    items.extend(rules::startup_high_impact(
        startup_high,
        startup_items.len(),
    ));

    // ---- Services ----
    let services = scan_services();
    let safe_running = services
        .iter()
        .filter(|svc| svc.classification == "Safe to change" && svc.status == "Running")
        .count();
    items.extend(rules::safe_running_services(safe_running, services.len()));

    // ---- Cleanup ----
    let reclaimable: u64 = crate::cleaner::scanner::scan_cleaner_targets()
        .iter()
        .map(|target| target.total_bytes)
        .sum();
    items.extend(rules::reclaimable_temp(reclaimable));

    // ---- Power ----
    let plans = list_plans();
    if let Some(kind) = active_plan_kind(&plans) {
        items.extend(rules::power_plan(
            kind,
            has_battery(),
            has_high_performance(&plans),
        ));
    }

    // ---- Privacy ----
    let privacy_items = scan_privacy_items();
    let unapplied = privacy_items.iter().filter(|item| !item.is_applied).count();
    items.extend(rules::privacy_unapplied(unapplied, privacy_items.len()));

    // ---- Security ----
    let security = check_security_health();
    items.extend(rules::defender_real_time_protection(
        security.real_time_protection,
    ));
    items.extend(rules::firewall_enabled(security.firewall_enabled));

    // ---- Windows Update ----
    let update = check_update_health();
    items.extend(rules::windows_update_service(
        update.service_state.as_deref(),
    ));
    items.extend(rules::pending_reboot(update.pending_reboot));

    // ---- Storage ----
    let disk = get_primary_disk_stats();
    items.extend(rules::disk_free_space(disk.free_bytes, disk.total_bytes));

    // ---- Applications ----
    // `check_updates` spawns winget and takes seconds, so it only runs when the
    // user enabled winget metadata scanning. Otherwise the update count is
    // unknown and the rule is given no measurement to report.
    let winget = winget_status();
    if winget.available && config.apps.scan_winget_metadata {
        let updates = check_updates().len();
        items.extend(rules::app_updates(true, updates));
    }

    sort_recommendations(&mut items);
    log_info(
        "recommendations",
        &format!("System analysis produced {} observation(s)", items.len()),
    );
    items
}

/// The kind of the plan Windows reports as active, if any plan is active.
fn active_plan_kind(plans: &[PowerPlan]) -> Option<PowerPlanKind> {
    plans
        .iter()
        .find(|plan| plan.is_active)
        .map(|plan| plan.kind)
}

/// Whether a High Performance plan is installed, which is what makes the
/// desktop advice actionable rather than a dead end.
fn has_high_performance(plans: &[PowerPlan]) -> bool {
    plans
        .iter()
        .any(|plan| plan.kind == PowerPlanKind::HighPerformance)
}

/// Whether this machine has a battery.
///
/// `GetSystemPowerStatus` sets bit 128 of `BatteryFlag` for "no system battery"
/// and reports 255 when it cannot tell. An undetermined flag returns `true` so
/// an unknown machine is treated as a possible laptop and the desktop-only
/// power advice stays quiet — Wino only says "desktop" about a machine that
/// said so.
fn has_battery() -> bool {
    const NO_SYSTEM_BATTERY: u8 = 128;
    const UNDETERMINED: u8 = 255;

    let mut status = SYSTEM_POWER_STATUS::default();
    // SAFETY: `status` is a valid, correctly sized SYSTEM_POWER_STATUS owned by
    // this frame for the duration of the call; the API only writes into it.
    if unsafe { GetSystemPowerStatus(&mut status) }.is_err() {
        return true;
    }

    if status.BatteryFlag == UNDETERMINED {
        return true;
    }
    status.BatteryFlag & NO_SYSTEM_BATTERY == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(guid: &str, kind: PowerPlanKind, is_active: bool) -> PowerPlan {
        PowerPlan {
            guid: guid.to_string(),
            name: format!("{:?}", kind),
            is_active,
            kind,
        }
    }

    #[test]
    fn active_plan_kind_comes_from_the_active_flag_only() {
        let plans = vec![
            plan("A", PowerPlanKind::HighPerformance, false),
            plan("B", PowerPlanKind::Balanced, true),
            plan("C", PowerPlanKind::Custom, false),
        ];
        assert_eq!(active_plan_kind(&plans), Some(PowerPlanKind::Balanced));
    }

    #[test]
    fn no_active_plan_yields_no_kind_and_no_advice() {
        let plans = vec![plan("A", PowerPlanKind::Balanced, false)];
        assert_eq!(active_plan_kind(&plans), None);
        assert!(active_plan_kind(&[]).is_none());
    }

    #[test]
    fn high_performance_availability_is_measured_not_assumed() {
        let without = vec![
            plan("A", PowerPlanKind::Balanced, true),
            plan("B", PowerPlanKind::PowerSaver, false),
        ];
        assert!(!has_high_performance(&without));

        let with = vec![
            plan("A", PowerPlanKind::Balanced, true),
            plan("C", PowerPlanKind::HighPerformance, false),
        ];
        assert!(has_high_performance(&with));
    }
}
