//! Before / after benchmark model.
//!
//! A sample records only values Wino can measure directly. There is
//! deliberately no "estimated improvement" field: the comparison view reports
//! measured deltas and nothing else.

use serde::{Deserialize, Serialize};

/// One captured system state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSample {
    /// Millisecond timestamp, unique per capture.
    pub id: String,
    /// User-facing label, e.g. "Before optimization".
    pub label: String,
    pub timestamp: String,

    // ---- Measured values ----
    /// Physical RAM in use, bytes.
    pub ram_used_bytes: u64,
    pub ram_total_bytes: u64,
    /// Running process count.
    pub process_count: usize,
    /// Number of startup entries Windows would launch.
    pub startup_count: usize,
    /// Total bytes in the temporary locations the Storage Cleaner targets.
    pub temp_bytes: u64,
    /// Friendly name of the active power plan, empty when it could not be read.
    pub power_plan: String,
    /// Service name to current state ("Running"/"Stopped"/"Unknown") for the
    /// services Wino can change.
    pub service_states: Vec<(String, String)>,
    /// Privacy rule id to applied flag.
    pub privacy_states: Vec<(String, bool)>,
}

impl BenchmarkSample {
    /// RAM in use as a percentage, or 0 when the total is unknown.
    pub fn ram_pct(&self) -> f32 {
        if self.ram_total_bytes == 0 {
            return 0.0;
        }
        (self.ram_used_bytes as f64 / self.ram_total_bytes as f64 * 100.0) as f32
    }

    /// Services that differ between two samples, as (name, before, after).
    pub fn service_diffs(&self, other: &BenchmarkSample) -> Vec<(String, String, String)> {
        let mut out = Vec::new();
        for (name, before) in &self.service_states {
            if let Some((_, after)) = other.service_states.iter().find(|(n, _)| n == name) {
                if before != after {
                    out.push((name.clone(), before.clone(), after.clone()));
                }
            }
        }
        out
    }

    /// Privacy rules that differ between two samples, as (id, before, after).
    pub fn privacy_diffs(&self, other: &BenchmarkSample) -> Vec<(String, bool, bool)> {
        let mut out = Vec::new();
        for (id, before) in &self.privacy_states {
            if let Some((_, after)) = other.privacy_states.iter().find(|(i, _)| i == id) {
                if before != after {
                    out.push((id.clone(), *before, *after));
                }
            }
        }
        out
    }
}

/// How a single metric changed between two samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaDirection {
    /// A numeric decrease (usually the improvement Wino is looking for).
    Decreased,
    Increased,
    Unchanged,
    /// The metric is textual, so only equality can be reported.
    Textual,
}

/// One row of the comparison view.
#[derive(Debug, Clone)]
pub struct MetricDelta {
    /// i18n key for the metric name.
    pub label_key: &'static str,
    pub before: String,
    pub after: String,
    pub direction: DeltaDirection,
}

/// Build a numeric delta row from raw values.
pub fn numeric_delta(label_key: &'static str, before: u64, after: u64, unit: &str) -> MetricDelta {
    let direction = match after.cmp(&before) {
        std::cmp::Ordering::Less => DeltaDirection::Decreased,
        std::cmp::Ordering::Greater => DeltaDirection::Increased,
        std::cmp::Ordering::Equal => DeltaDirection::Unchanged,
    };
    MetricDelta {
        label_key,
        before: format!("{} {}", before, unit).trim().to_string(),
        after: format!("{} {}", after, unit).trim().to_string(),
        direction,
    }
}

/// Build a textual delta row (power plan name, and similar).
pub fn textual_delta(label_key: &'static str, before: &str, after: &str) -> MetricDelta {
    MetricDelta {
        label_key,
        before: before.to_string(),
        after: after.to_string(),
        direction: if before == after {
            DeltaDirection::Unchanged
        } else {
            DeltaDirection::Textual
        },
    }
}

/// Human delta string for a byte metric ("-0.5 GB", "+120 MB", "no change").
pub fn byte_delta_label(before: u64, after: u64) -> String {
    if before == after {
        return "no change".to_string();
    }
    let diff = if after > before {
        (after - before) as f64
    } else {
        -((before - after) as f64)
    };
    let sign = if diff > 0.0 { "+" } else { "-" };
    let magnitude = diff.abs();
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = MB * 1024.0;
    if magnitude >= GB {
        format!("{}{:.2} GB", sign, magnitude / GB)
    } else {
        format!("{}{:.0} MB", sign, magnitude / MB)
    }
}

/// Compare two samples into display rows.
///
/// `before` is the earlier capture. Rows are emitted in a fixed order so the
/// table does not reshuffle between comparisons.
pub fn compare(before: &BenchmarkSample, after: &BenchmarkSample) -> Vec<MetricDelta> {
    let mut rows = vec![MetricDelta {
        label_key: "bench.idle_ram",
        before: crate::apps::models::format_size(before.ram_used_bytes),
        after: crate::apps::models::format_size(after.ram_used_bytes),
        direction: match after.ram_used_bytes.cmp(&before.ram_used_bytes) {
            std::cmp::Ordering::Less => DeltaDirection::Decreased,
            std::cmp::Ordering::Greater => DeltaDirection::Increased,
            std::cmp::Ordering::Equal => DeltaDirection::Unchanged,
        },
    }];

    rows.push(numeric_delta(
        "bench.process_count",
        before.process_count as u64,
        after.process_count as u64,
        "",
    ));

    rows.push(numeric_delta(
        "bench.startup_count",
        before.startup_count as u64,
        after.startup_count as u64,
        "",
    ));

    rows.push(MetricDelta {
        label_key: "bench.temp_usage",
        before: crate::apps::models::format_size(before.temp_bytes),
        after: crate::apps::models::format_size(after.temp_bytes),
        direction: match after.temp_bytes.cmp(&before.temp_bytes) {
            std::cmp::Ordering::Less => DeltaDirection::Decreased,
            std::cmp::Ordering::Greater => DeltaDirection::Increased,
            std::cmp::Ordering::Equal => DeltaDirection::Unchanged,
        },
    });

    rows.push(textual_delta(
        "bench.power_plan",
        &before.power_plan,
        &after.power_plan,
    ));

    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(ram_used: u64, procs: usize, startup: usize, temp: u64) -> BenchmarkSample {
        BenchmarkSample {
            id: "1".to_string(),
            label: "test".to_string(),
            timestamp: "2026-01-01 00:00:00".to_string(),
            ram_used_bytes: ram_used,
            ram_total_bytes: 16 * 1024 * 1024 * 1024,
            process_count: procs,
            startup_count: startup,
            temp_bytes: temp,
            power_plan: "Balanced".to_string(),
            service_states: vec![("DiagTrack".to_string(), "Running".to_string())],
            privacy_states: vec![("privacy_advertising_id".to_string(), false)],
        }
    }

    #[test]
    fn ram_percentage_handles_unknown_total() {
        let mut s = sample(8 * 1024 * 1024 * 1024, 100, 10, 0);
        assert!((s.ram_pct() - 50.0).abs() < 0.01);
        s.ram_total_bytes = 0;
        assert_eq!(s.ram_pct(), 0.0);
    }

    #[test]
    fn byte_delta_label_reports_direction_and_units() {
        let gb = 1024u64 * 1024 * 1024;
        assert_eq!(byte_delta_label(5 * gb, 4 * gb), "-1.00 GB");
        assert_eq!(byte_delta_label(4 * gb, 5 * gb), "+1.00 GB");
        assert_eq!(byte_delta_label(100, 100), "no change");
        assert_eq!(byte_delta_label(0, 120 * 1024 * 1024), "+120 MB");
    }

    #[test]
    fn compare_marks_decreases_and_increases() {
        let before = sample(5 * 1024 * 1024 * 1024, 120, 17, 8 * 1024 * 1024 * 1024);
        let after = sample(4 * 1024 * 1024 * 1024, 100, 11, 2 * 1024 * 1024 * 1024);
        let rows = compare(&before, &after);

        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0].label_key, "bench.idle_ram");
        assert_eq!(rows[0].direction, DeltaDirection::Decreased);
        assert_eq!(rows[1].direction, DeltaDirection::Decreased);
        assert_eq!(rows[2].direction, DeltaDirection::Decreased);
        assert_eq!(rows[3].direction, DeltaDirection::Decreased);
        assert_eq!(rows[4].direction, DeltaDirection::Unchanged);
    }

    #[test]
    fn service_and_privacy_diffs_are_reported() {
        let before = sample(1, 1, 1, 1);
        let mut after = sample(1, 1, 1, 1);
        after.service_states = vec![("DiagTrack".to_string(), "Stopped".to_string())];
        after.privacy_states = vec![("privacy_advertising_id".to_string(), true)];

        let svc = before.service_diffs(&after);
        assert_eq!(
            svc,
            vec![(
                "DiagTrack".to_string(),
                "Running".to_string(),
                "Stopped".to_string()
            )]
        );

        let priv_diffs = before.privacy_diffs(&after);
        assert_eq!(
            priv_diffs,
            vec![("privacy_advertising_id".to_string(), false, true)]
        );
    }
}
