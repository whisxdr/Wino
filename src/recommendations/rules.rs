//! Threshold rules behind the Recommendations view.
//!
//! Every function here is pure: it receives already-measured numbers and either
//! returns one observation or `None` when the measurement is unremarkable.
//! Nothing in this file reads the system, which is what keeps the wording tied
//! to a real measurement — a rule can only state a number it was handed — and
//! makes every threshold testable without a machine.
//!
//! Absence of a measurement is not evidence of a problem. A rule given a value
//! it cannot interpret (an unknown service state, an empty scanner result)
//! returns `None` rather than a zero-value observation.

use crate::power::models::PowerPlanKind;
use crate::recommendations::models::{
    gb, Recommendation, RecommendationArea, RecommendationSeverity, ReviewTarget,
};

/// RAM in use at or above this percentage is High severity.
pub const RAM_HIGH_PCT: f32 = 85.0;
/// RAM in use at or above this percentage (but below [`RAM_HIGH_PCT`]) is Medium.
pub const RAM_MEDIUM_PCT: f32 = 75.0;
/// Commit charge above this fraction of the commit limit is Medium.
pub const COMMIT_ELEVATED_RATIO: f64 = 0.80;
/// This many high-impact startup entries is Medium.
pub const STARTUP_HIGH_IMPACT_MIN: usize = 3;
/// This many running "Safe to change" services is Low.
pub const SERVICES_SAFE_RUNNING_MIN: usize = 3;
/// Reclaimable temporary data at or above this is Medium.
pub const CLEANUP_MEDIUM_BYTES: u64 = 2 * 1024 * 1024 * 1024;
/// Reclaimable temporary data at or above this (but below Medium) is Low.
pub const CLEANUP_LOW_BYTES: u64 = 500 * 1024 * 1024;
/// This many unapplied privacy rules is Low.
pub const PRIVACY_UNAPPLIED_MIN: usize = 3;
/// Free space below this percentage of the drive is Medium.
pub const DISK_LOW_FREE_PCT: f32 = 10.0;
/// Free space below this percentage of the drive is High.
pub const DISK_CRITICAL_FREE_PCT: f32 = 5.0;

/// Percentage helper shared by the rules that report a measured ratio.
///
/// One decimal, never rounded to a whole percent: the thresholds sit on whole
/// numbers, so a value of 84.7% rendered as "85%" would carry a Medium severity
/// while reading as the High boundary. The displayed figure is the measured
/// figure.
fn pct(part: u64, whole: u64) -> String {
    if whole == 0 {
        return "0.0".to_string();
    }
    format!("{:.1}", part as f64 / whole as f64 * 100.0)
}

/// RAM usage. High at or above 85%, Medium at or above 75%.
///
/// `usage_pct` is the figure the memory scanner measured; it is reported as
/// measured rather than recomputed, so the headline and the detail line can
/// never disagree.
pub fn ram_usage(usage_pct: f32, used_bytes: u64, total_bytes: u64) -> Option<Recommendation> {
    if total_bytes == 0 {
        return None;
    }
    let severity = if usage_pct >= RAM_HIGH_PCT {
        RecommendationSeverity::High
    } else if usage_pct >= RAM_MEDIUM_PCT {
        RecommendationSeverity::Medium
    } else {
        return None;
    };

    Some(Recommendation::new(
        "memory.ram_usage",
        RecommendationArea::Memory,
        severity,
        &format!("Memory usage is at {:.1}%.", usage_pct),
        &format!(
            "{} of {} ({}%)",
            gb(used_bytes),
            gb(total_bytes),
            pct(used_bytes, total_bytes)
        ),
        ReviewTarget::Memory,
    ))
}

/// Commit charge above 80% of the commit limit.
pub fn commit_charge(commit_used_bytes: u64, commit_limit_bytes: u64) -> Option<Recommendation> {
    if commit_limit_bytes == 0 {
        return None;
    }
    let ratio = commit_used_bytes as f64 / commit_limit_bytes as f64;
    if ratio <= COMMIT_ELEVATED_RATIO {
        return None;
    }

    Some(Recommendation::new(
        "memory.commit_charge",
        RecommendationArea::Memory,
        RecommendationSeverity::Medium,
        &format!(
            "Commit charge is at {:.1}% of the commit limit.",
            ratio * 100.0
        ),
        &format!(
            "{} of {} committed ({}%)",
            gb(commit_used_bytes),
            gb(commit_limit_bytes),
            pct(commit_used_bytes, commit_limit_bytes)
        ),
        ReviewTarget::Memory,
    ))
}

/// Startup entries Windows launches at sign-in that were estimated high impact.
pub fn startup_high_impact(high_impact: usize, total: usize) -> Option<Recommendation> {
    if high_impact < STARTUP_HIGH_IMPACT_MIN {
        return None;
    }

    Some(Recommendation::new(
        "startup.high_impact",
        RecommendationArea::Startup,
        RecommendationSeverity::Medium,
        &format!("{} startup applications have high impact.", high_impact),
        &format!("{} of {} startup entries", high_impact, total),
        ReviewTarget::Startup,
    ))
}

/// Services classified "Safe to change" that are currently running.
pub fn safe_running_services(count: usize, total: usize) -> Option<Recommendation> {
    if count < SERVICES_SAFE_RUNNING_MIN {
        return None;
    }

    Some(Recommendation::new(
        "services.safe_running",
        RecommendationArea::Services,
        RecommendationSeverity::Low,
        &format!("{} services marked safe to change are running.", count),
        &format!("{} of {} services", count, total),
        ReviewTarget::Services,
    ))
}

/// Reclaimable temporary data reported by the Storage Cleaner scan.
///
/// The same measurement the cleaner itself reports, so the number here and the
/// number in that view are the same number.
pub fn reclaimable_temp(total_bytes: u64) -> Option<Recommendation> {
    let severity = if total_bytes >= CLEANUP_MEDIUM_BYTES {
        RecommendationSeverity::Medium
    } else if total_bytes >= CLEANUP_LOW_BYTES {
        RecommendationSeverity::Low
    } else {
        return None;
    };

    Some(Recommendation::new(
        "cleanup.reclaimable",
        RecommendationArea::Cleanup,
        severity,
        &format!(
            "{} of temporary data can potentially be cleaned.",
            gb(total_bytes)
        ),
        &format!("{} recoverable in the cleaner targets", gb(total_bytes)),
        ReviewTarget::Cleaner,
    ))
}

/// Power plan advice. Advisory only, and deliberately quiet: Info, never a
/// warning, because a plan choice is a preference and not a fault.
///
/// `has_battery` and `high_performance_available` are measured facts about the
/// machine. On a laptop the Balanced plan is the right default, so no
/// observation is emitted there.
pub fn power_plan(
    active: PowerPlanKind,
    has_battery: bool,
    high_performance_available: bool,
) -> Option<Recommendation> {
    match active {
        PowerPlanKind::Balanced if !has_battery && high_performance_available => {
            Some(Recommendation::new(
                "power.balanced_on_desktop",
                RecommendationArea::Power,
                RecommendationSeverity::Info,
                "This desktop is running the Balanced power plan; a High Performance plan is available.",
                "Active plan: Balanced, no battery detected, High Performance present",
                ReviewTarget::Power,
            ))
        }
        PowerPlanKind::HighPerformance => Some(Recommendation::new(
            "power.high_performance_active",
            RecommendationArea::Power,
            RecommendationSeverity::Info,
            "The High Performance power plan is active.",
            "Active plan: High Performance",
            ReviewTarget::Power,
        )),
        _ => None,
    }
}

/// Privacy rules that were measured as not applied.
pub fn privacy_unapplied(count: usize, total: usize) -> Option<Recommendation> {
    if count < PRIVACY_UNAPPLIED_MIN {
        return None;
    }

    Some(Recommendation::new(
        "privacy.not_applied",
        RecommendationArea::Privacy,
        RecommendationSeverity::Low,
        &format!("{} privacy settings are not applied.", count),
        &format!("{} of {} privacy rules are not applied", count, total),
        ReviewTarget::Privacy,
    ))
}

/// Defender real-time protection measured as off.
pub fn defender_real_time_protection(real_time_on: bool) -> Option<Recommendation> {
    if real_time_on {
        return None;
    }

    Some(Recommendation::new(
        "security.defender_realtime",
        RecommendationArea::Security,
        RecommendationSeverity::High,
        "Microsoft Defender real-time protection is off.",
        "Real-time protection: off",
        ReviewTarget::Security,
    ))
}

/// Windows Firewall measured as off for the standard profile.
pub fn firewall_enabled(enabled: bool) -> Option<Recommendation> {
    if enabled {
        return None;
    }

    Some(Recommendation::new(
        "security.firewall",
        RecommendationArea::Security,
        RecommendationSeverity::High,
        "The Windows Firewall is off for the standard profile.",
        "Firewall (standard profile): off",
        ReviewTarget::Security,
    ))
}

/// Windows Update service measured as stopped.
///
/// `None` means the state could not be queried. That is *not* evidence of a
/// problem, so it produces no observation at all.
pub fn windows_update_service(service_state: Option<&str>) -> Option<Recommendation> {
    if service_state != Some("Stopped") {
        return None;
    }

    Some(Recommendation::new(
        "updates.service_stopped",
        RecommendationArea::Updates,
        RecommendationSeverity::Medium,
        "The Windows Update service (wuauserv) is stopped.",
        "wuauserv: Stopped",
        ReviewTarget::Health,
    ))
}

/// A restart is pending.
pub fn pending_reboot(pending: bool) -> Option<Recommendation> {
    if !pending {
        return None;
    }

    Some(Recommendation::new(
        "updates.pending_reboot",
        RecommendationArea::Updates,
        RecommendationSeverity::Low,
        "Windows is waiting for a restart.",
        "Pending restart: yes",
        ReviewTarget::Health,
    ))
}

/// Free space on the primary drive. High below 5%, Medium below 10%.
pub fn disk_free_space(free_bytes: u64, total_bytes: u64) -> Option<Recommendation> {
    if total_bytes == 0 {
        return None;
    }
    let free_pct = (free_bytes as f64 / total_bytes as f64 * 100.0) as f32;

    let severity = if free_pct < DISK_CRITICAL_FREE_PCT {
        RecommendationSeverity::High
    } else if free_pct < DISK_LOW_FREE_PCT {
        RecommendationSeverity::Medium
    } else {
        return None;
    };

    Some(Recommendation::new(
        "storage.low_free_space",
        RecommendationArea::Storage,
        severity,
        &format!("Only {:.1}% of the system drive is free.", free_pct),
        &format!(
            "{} free of {} ({}%)",
            gb(free_bytes),
            gb(total_bytes),
            pct(free_bytes, total_bytes)
        ),
        ReviewTarget::Storage,
    ))
}

/// Application updates reported by winget.
///
/// Requires both a usable winget and at least one updatable package; winget
/// being unavailable is a missing tool, not a machine problem worth reporting.
pub fn app_updates(winget_available: bool, update_count: usize) -> Option<Recommendation> {
    if !winget_available || update_count == 0 {
        return None;
    }

    let verb = if update_count == 1 {
        "update is"
    } else {
        "updates are"
    };
    Some(Recommendation::new(
        "apps.updates_available",
        RecommendationArea::Updates,
        RecommendationSeverity::Low,
        &format!("{} application {} available.", update_count, verb),
        &format!("{} upgradable packages reported by winget", update_count),
        ReviewTarget::Apps,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const GB: u64 = 1024 * 1024 * 1024;

    #[test]
    fn ram_thresholds_split_at_75_and_85_percent() {
        let total = 16 * GB;

        assert!(ram_usage(74.9, 11 * GB, total).is_none());
        assert_eq!(
            ram_usage(75.0, 12 * GB, total).map(|r| r.severity),
            Some(RecommendationSeverity::Medium)
        );
        assert_eq!(
            ram_usage(84.9, 13 * GB, total).map(|r| r.severity),
            Some(RecommendationSeverity::Medium)
        );
        assert_eq!(
            ram_usage(85.0, 14 * GB, total).map(|r| r.severity),
            Some(RecommendationSeverity::High)
        );

        let high = ram_usage(87.4, 14 * GB, total).expect("high ram observation");
        assert!(
            high.title.contains("87"),
            "title must state the measurement: {}",
            high.title
        );
        assert_eq!(high.measured, "14.0 GB of 16.0 GB (87.5%)");
        assert_eq!(high.target, ReviewTarget::Memory);
        assert_eq!(high.area, RecommendationArea::Memory);
    }

    #[test]
    fn a_near_boundary_reading_is_not_rounded_up_into_the_next_severity() {
        // 84.7% must not display as "85%" while being Medium, or the severity
        // and the number would contradict each other.
        let medium =
            ram_usage(84.7, 847 * 1024 * 1024, 1000 * 1024 * 1024).expect("medium ram observation");
        assert_eq!(medium.severity, RecommendationSeverity::Medium);
        assert_eq!(medium.title, "Memory usage is at 84.7%.");
        assert_eq!(medium.measured, "0.8 GB of 1.0 GB (84.7%)");
    }

    #[test]
    fn ram_rule_refuses_an_unknown_total() {
        assert!(ram_usage(99.0, 8 * GB, 0).is_none());
    }

    #[test]
    fn commit_charge_fires_only_above_80_percent() {
        let limit = 20 * GB;
        assert!(commit_charge(16 * GB, limit).is_none());
        assert!(commit_charge(16 * GB + 1, limit).is_some());
        assert!(commit_charge(5 * GB, 0).is_none());

        let rec = commit_charge(17 * GB, limit).expect("commit observation");
        assert_eq!(rec.severity, RecommendationSeverity::Medium);
        assert!(rec.measured.contains("17.0 GB"), "{}", rec.measured);
        assert_eq!(rec.measured, "17.0 GB of 20.0 GB committed (85.0%)");
    }

    #[test]
    fn startup_rule_needs_three_high_impact_entries() {
        assert!(startup_high_impact(2, 9).is_none());

        let rec = startup_high_impact(5, 12).expect("startup observation");
        assert_eq!(rec.severity, RecommendationSeverity::Medium);
        assert_eq!(rec.title, "5 startup applications have high impact.");
        assert_eq!(rec.measured, "5 of 12 startup entries");
        assert_eq!(rec.target, ReviewTarget::Startup);

        // Three is the threshold, so it fires on exactly three as well.
        assert!(startup_high_impact(3, 3).is_some());
    }

    #[test]
    fn services_rule_needs_three_safe_running_services() {
        assert!(safe_running_services(2, 40).is_none());

        let rec = safe_running_services(3, 40).expect("services observation");
        assert_eq!(rec.severity, RecommendationSeverity::Low);
        assert!(rec.title.starts_with("3 services"));
        assert_eq!(rec.measured, "3 of 40 services");
    }

    #[test]
    fn cleanup_rule_steps_from_low_to_medium_at_two_gigabytes() {
        assert!(reclaimable_temp(CLEANUP_LOW_BYTES - 1).is_none());
        assert_eq!(
            reclaimable_temp(CLEANUP_LOW_BYTES).map(|r| r.severity),
            Some(RecommendationSeverity::Low)
        );
        assert_eq!(
            reclaimable_temp(CLEANUP_MEDIUM_BYTES - 1).map(|r| r.severity),
            Some(RecommendationSeverity::Low)
        );
        assert_eq!(
            reclaimable_temp(CLEANUP_MEDIUM_BYTES).map(|r| r.severity),
            Some(RecommendationSeverity::Medium)
        );

        let rec = reclaimable_temp(78 * GB / 10).expect("cleanup observation");
        assert_eq!(
            rec.title,
            "7.8 GB of temporary data can potentially be cleaned."
        );
        assert_eq!(rec.target, ReviewTarget::Cleaner);
    }

    #[test]
    fn power_rule_stays_quiet_on_batteries_and_off_balanced_desktops() {
        // A laptop on Balanced is the expected default: nothing to say.
        assert!(power_plan(PowerPlanKind::Balanced, true, true).is_none());
        // A desktop on Balanced with no High Performance plan installed.
        assert!(power_plan(PowerPlanKind::Balanced, false, false).is_none());
        // Custom and Power Saver plans are user choices, not observations.
        assert!(power_plan(PowerPlanKind::Custom, false, true).is_none());
        assert!(power_plan(PowerPlanKind::PowerSaver, false, true).is_none());

        let desktop = power_plan(PowerPlanKind::Balanced, false, true).expect("power observation");
        assert_eq!(desktop.severity, RecommendationSeverity::Info);
        assert!(desktop.title.contains("High Performance"));

        let active = power_plan(PowerPlanKind::HighPerformance, true, true).expect("active plan");
        assert_eq!(active.severity, RecommendationSeverity::Info);
        assert_eq!(active.measured, "Active plan: High Performance");
    }

    #[test]
    fn privacy_rule_needs_three_unapplied_rules() {
        assert!(privacy_unapplied(2, 30).is_none());

        let rec = privacy_unapplied(7, 30).expect("privacy observation");
        assert_eq!(rec.severity, RecommendationSeverity::Low);
        assert_eq!(rec.title, "7 privacy settings are not applied.");
        assert_eq!(rec.target, ReviewTarget::Privacy);
    }

    #[test]
    fn security_rules_fire_only_on_a_measured_off_state() {
        assert!(defender_real_time_protection(true).is_none());
        assert!(firewall_enabled(true).is_none());

        let rtp = defender_real_time_protection(false).expect("rtp observation");
        assert_eq!(rtp.severity, RecommendationSeverity::High);
        assert_eq!(rtp.target, ReviewTarget::Security);

        let fw = firewall_enabled(false).expect("firewall observation");
        assert_eq!(fw.severity, RecommendationSeverity::High);
        assert_eq!(fw.id, "security.firewall");
    }

    #[test]
    fn update_rule_ignores_an_unqueryable_service_state() {
        assert!(windows_update_service(None).is_none());
        assert!(windows_update_service(Some("Running")).is_none());
        assert!(windows_update_service(Some("Unknown")).is_none());

        let rec = windows_update_service(Some("Stopped")).expect("update observation");
        assert_eq!(rec.severity, RecommendationSeverity::Medium);
        assert_eq!(rec.area, RecommendationArea::Updates);
        assert_eq!(rec.target, ReviewTarget::Health);
    }

    #[test]
    fn pending_reboot_is_low_severity_and_health_targeted() {
        assert!(pending_reboot(false).is_none());

        let rec = pending_reboot(true).expect("reboot observation");
        assert_eq!(rec.severity, RecommendationSeverity::Low);
        assert_eq!(rec.target, ReviewTarget::Health);
    }

    #[test]
    fn disk_rule_splits_at_five_and_ten_percent_free() {
        let total = 500 * GB;
        assert!(disk_free_space(50 * GB, total).is_none());
        assert_eq!(
            disk_free_space(49 * GB, total).map(|r| r.severity),
            Some(RecommendationSeverity::Medium)
        );
        assert_eq!(
            disk_free_space(25 * GB, total).map(|r| r.severity),
            Some(RecommendationSeverity::Medium)
        );
        assert_eq!(
            disk_free_space(24 * GB, total).map(|r| r.severity),
            Some(RecommendationSeverity::High)
        );
        assert!(disk_free_space(0, 0).is_none());

        let rec = disk_free_space(24 * GB, total).expect("disk observation");
        assert_eq!(rec.title, "Only 4.8% of the system drive is free.");
        assert_eq!(rec.measured, "24.0 GB free of 500.0 GB (4.8%)");
    }

    #[test]
    fn app_update_rule_requires_winget_and_a_real_update() {
        assert!(app_updates(false, 12).is_none());
        assert!(app_updates(true, 0).is_none());

        let rec = app_updates(true, 12).expect("apps observation");
        assert_eq!(rec.severity, RecommendationSeverity::Low);
        assert_eq!(rec.title, "12 application updates are available.");
        assert_eq!(rec.target, ReviewTarget::Apps);

        let single = app_updates(true, 1).expect("single update");
        assert_eq!(single.title, "1 application update is available.");
    }

    #[test]
    fn every_rule_id_is_unique_so_a_rescan_cannot_duplicate_a_row() {
        let mut ids = vec![
            "memory.ram_usage",
            "memory.commit_charge",
            "startup.high_impact",
            "services.safe_running",
            "cleanup.reclaimable",
            "power.balanced_on_desktop",
            "power.high_performance_active",
            "privacy.not_applied",
            "security.defender_realtime",
            "security.firewall",
            "updates.service_stopped",
            "updates.pending_reboot",
            "storage.low_free_space",
            "apps.updates_available",
        ];
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total);
    }
}
