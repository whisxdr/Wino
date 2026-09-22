//! Benchmark capture: take one read-only snapshot of measured system state.
//!
//! A capture exists so a before/after comparison can be made of numbers Wino
//! actually measured, never of estimates. Every field comes from the same
//! scanner the corresponding view uses — the temporary-data figure is the
//! Storage Cleaner's own measurement, the service states are the Services
//! view's own enumeration — so the comparison is comparable by construction.
//!
//! Capture is strictly read-only. It opens no key for writing, spawns no
//! mutating process, and changes no setting; the only subprocess involved is
//! the read-only power-plan enumeration, which runs through the native API
//! rather than a shell.
//!
//! The per-service and per-rule lists are capped so a capture stays a small
//! JSON record that a machine with hundreds of services cannot bloat.

use crate::benchmark::models::BenchmarkSample;
use crate::core::logger::log_info;
use crate::memory::monitor::capture_memory_snapshot;
use crate::monitoring::ram::RamStats;
use crate::power::manager::list_plans;
use chrono::Local;

/// Upper bound on the service and privacy lists inside one capture.
pub const MAX_STATE_ENTRIES: usize = 50;

/// Capture the current system state under a user-facing label.
pub fn capture_sample(label: &str) -> BenchmarkSample {
    let now = Local::now();
    let stats = capture_memory_snapshot().stats;
    let startup_count = crate::startup::scanner::scan_startup_items().len();
    let temp_bytes = temp_bytes_total();
    let power_plan = active_plan_name();
    let service_states = measurable_service_states();
    let privacy_states = privacy_states();

    let sample = build_sample(SampleInputs {
        id: capture_id(now.timestamp_millis(), now.timestamp_subsec_millis()),
        label,
        timestamp: now.format("%Y-%m-%d %H:%M:%S").to_string(),
        stats: &stats,
        startup_count,
        temp_bytes,
        power_plan,
        service_states,
        privacy_states,
    });

    log_info(
        "benchmark",
        &format!(
            "Captured benchmark '{}' ({} MB RAM, {} processes, {} startup entries, {} MB temp)",
            sample.label,
            sample.ram_used_bytes / (1024 * 1024),
            sample.process_count,
            sample.startup_count,
            sample.temp_bytes / (1024 * 1024)
        ),
    );
    sample
}

/// Total bytes in the temporary locations the Storage Cleaner targets.
///
/// Summed from [`crate::cleaner::scanner::scan_cleaner_targets`], the same scan
/// the cleaner view reports, so a before/after pair measures one quantity.
pub fn temp_bytes_total() -> u64 {
    crate::cleaner::scanner::scan_cleaner_targets()
        .iter()
        .map(|item| item.total_bytes)
        .sum()
}

/// Assemble a sample from already-measured values.
///
/// Split out from [`capture_sample`] so the arithmetic and the capping rules are
/// testable with fixed inputs and no machine access.
/// Inputs for one capture, grouped so the builder does not take nine positional
/// arguments (where a transposed pair would compile and produce wrong data).
struct SampleInputs<'a> {
    id: String,
    label: &'a str,
    timestamp: String,
    stats: &'a RamStats,
    startup_count: usize,
    temp_bytes: u64,
    power_plan: String,
    service_states: Vec<(String, String)>,
    privacy_states: Vec<(String, bool)>,
}

/// Assemble a sample from measured inputs, capping the per-service and
/// per-privacy lists so a capture stays small.
fn build_sample(inputs: SampleInputs<'_>) -> BenchmarkSample {
    let mut capped_services = inputs.service_states;
    capped_services.truncate(MAX_STATE_ENTRIES);
    let mut capped_privacy = inputs.privacy_states;
    capped_privacy.truncate(MAX_STATE_ENTRIES);

    BenchmarkSample {
        id: inputs.id,
        label: inputs.label.to_string(),
        timestamp: inputs.timestamp,
        ram_used_bytes: inputs.stats.used_bytes,
        ram_total_bytes: inputs.stats.total_bytes,
        process_count: inputs.stats.process_count,
        startup_count: inputs.startup_count,
        temp_bytes: inputs.temp_bytes,
        power_plan: inputs.power_plan,
        service_states: capped_services,
        privacy_states: capped_privacy,
    }
}

/// Capture id: millisecond epoch in hex, plus sub-millisecond fraction.
///
/// Same shape the snapshot module uses, so the two id spaces look alike and two
/// captures taken in the same millisecond still get distinct ids.
fn capture_id(millis: i64, subsec_millis: u32) -> String {
    format!("{:x}-{}", millis, subsec_millis)
}

/// Friendly name of the active power plan, or an empty string when it cannot be
/// read. Never a placeholder name: an unreadable plan must be visibly absent
/// from the comparison rather than shown as a fabricated value.
fn active_plan_name() -> String {
    list_plans()
        .into_iter()
        .find(|plan| plan.is_active)
        .map(|plan| plan.name)
        .unwrap_or_default()
}

/// States of the services Wino can actually change.
///
/// Only "Safe to change" and "Optional" are recorded: those are the services a
/// Wino optimization can move, so they are the only ones whose state can differ
/// meaningfully between two captures.
fn measurable_service_states() -> Vec<(String, String)> {
    crate::services::scanner::scan_services()
        .into_iter()
        .filter(|svc| svc.classification == "Safe to change" || svc.classification == "Optional")
        .map(|svc| (svc.service_name, svc.status))
        .collect()
}

/// Applied state of every privacy rule, as (rule id, applied).
fn privacy_states() -> Vec<(String, bool)> {
    crate::privacy::scanner::scan_privacy_items()
        .into_iter()
        .map(|item| (item.rule.id, item.is_applied))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(used: u64, total: u64, processes: usize) -> RamStats {
        RamStats {
            total_bytes: total,
            used_bytes: used,
            process_count: processes,
            ..RamStats::default()
        }
    }

    fn sample_from(stats: &RamStats, services: usize, privacy: usize) -> BenchmarkSample {
        build_sample(SampleInputs {
            id: capture_id(1_700_000_000_000, 250),
            label: "Before optimization",
            timestamp: "2026-01-02 03:04:05".to_string(),
            stats,
            startup_count: 17,
            temp_bytes: 8 * 1024 * 1024 * 1024,
            power_plan: "Balanced".to_string(),
            service_states: (0..services)
                .map(|i| (format!("svc{}", i), "Running".to_string()))
                .collect(),
            privacy_states: (0..privacy)
                .map(|i| (format!("rule{}", i), false))
                .collect(),
        })
    }

    #[test]
    fn sample_from_fixed_inputs_reports_the_expected_ram_percentage() {
        let gb = 1024u64 * 1024 * 1024;
        let sample = sample_from(&stats(5 * gb, 16 * gb, 123), 2, 3);

        assert_eq!(sample.ram_used_bytes, 5 * gb);
        assert_eq!(sample.ram_total_bytes, 16 * gb);
        assert_eq!(sample.process_count, 123);
        assert_eq!(sample.startup_count, 17);
        assert_eq!(sample.temp_bytes, 8 * gb);
        assert!((sample.ram_pct() - 31.25).abs() < 0.01);
    }

    #[test]
    fn unknown_ram_total_does_not_produce_a_fabricated_percentage() {
        let sample = sample_from(&stats(0, 0, 0), 0, 0);
        assert_eq!(sample.ram_pct(), 0.0);
    }

    #[test]
    fn state_lists_are_capped_so_a_capture_stays_small() {
        let gb = 1024u64 * 1024 * 1024;
        let sample = sample_from(&stats(4 * gb, 16 * gb, 100), 400, 400);

        assert_eq!(sample.service_states.len(), MAX_STATE_ENTRIES);
        assert_eq!(sample.privacy_states.len(), MAX_STATE_ENTRIES);
        // The cap keeps the first entries, so the list is a prefix, not a sample.
        assert_eq!(sample.service_states[0].0, "svc0");
        assert_eq!(sample.privacy_states[0].0, "rule0");
    }

    #[test]
    fn short_state_lists_are_kept_whole() {
        let gb = 1024u64 * 1024 * 1024;
        let sample = sample_from(&stats(4 * gb, 16 * gb, 100), 7, 9);
        assert_eq!(sample.service_states.len(), 7);
        assert_eq!(sample.privacy_states.len(), 9);
    }

    #[test]
    fn capture_id_is_hex_milliseconds_with_a_fraction_suffix() {
        let id = capture_id(1_700_000_000_000, 250);
        assert_eq!(id, "18bcfe56800-250");

        let (millis, subsec) = id.split_once('-').expect("id has a fraction separator");
        assert!(
            u64::from_str_radix(millis, 16).is_ok(),
            "id prefix must be hex: {}",
            millis
        );
        assert!(
            subsec.parse::<u32>().is_ok(),
            "suffix must be decimal: {}",
            subsec
        );

        // Two captures inside the same millisecond still get distinct ids.
        assert_ne!(
            capture_id(1_700_000_000_000, 1),
            capture_id(1_700_000_000_000, 2)
        );
    }

    #[test]
    fn label_is_stored_verbatim() {
        let gb = 1024u64 * 1024 * 1024;
        let sample = sample_from(&stats(gb, 2 * gb, 1), 0, 0);
        assert_eq!(sample.label, "Before optimization");
        assert_eq!(sample.timestamp, "2026-01-02 03:04:05");
    }

    #[test]
    fn timestamp_format_matches_the_snapshot_module_shape() {
        let formatted = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        assert_eq!(formatted.len(), 19, "unexpected timestamp: {}", formatted);
        assert_eq!(formatted.matches('-').count(), 2);
        assert_eq!(formatted.matches(':').count(), 2);
    }
}
