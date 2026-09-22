//! System Health Center.
//!
//! Every check reports the state it actually observed. When a check cannot be
//! queried, the result is [`HealthState::Unknown`] — never a fabricated
//! healthy status. This is the one rule that matters most in this module: a
//! green dashboard that lies is worse than a grey one that admits it does not
//! know.

use crate::health::defender::check_security_health;
use crate::health::integrity::{interpret_sfc_output, IntegrityOutcome};
use crate::monitoring::disk::get_primary_disk_stats;
use crate::services::scanner::service_state_query;
use serde::{Deserialize, Serialize};

/// State of one health check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum HealthState {
    Healthy,
    Attention,
    Warning,
    Critical,
    Unknown,
}

impl HealthState {
    pub fn i18n_key(&self) -> &'static str {
        match self {
            HealthState::Healthy => "healthcenter.state_healthy",
            HealthState::Attention => "healthcenter.state_attention",
            HealthState::Warning => "healthcenter.state_warning",
            HealthState::Critical => "healthcenter.state_critical",
            HealthState::Unknown => "healthcenter.state_unknown",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            HealthState::Healthy => "HEALTHY",
            HealthState::Attention => "ATTENTION",
            HealthState::Warning => "WARNING",
            HealthState::Critical => "CRITICAL",
            HealthState::Unknown => "UNKNOWN",
        }
    }

    pub fn color_rgb(&self) -> (u8, u8, u8) {
        match self {
            HealthState::Healthy => (34, 197, 94),
            HealthState::Attention => (234, 179, 8),
            HealthState::Warning => (249, 115, 22),
            HealthState::Critical => (239, 68, 68),
            HealthState::Unknown => (148, 163, 184),
        }
    }

    /// Rank used when rolling individual checks into an overall verdict.
    /// `Unknown` ranks below every real problem but above `Healthy`, so an
    /// unqueryable check never turns the dashboard green on its own.
    fn severity_rank(&self) -> u8 {
        match self {
            HealthState::Critical => 4,
            HealthState::Warning => 3,
            HealthState::Attention => 2,
            HealthState::Unknown => 1,
            HealthState::Healthy => 0,
        }
    }
}

/// One named check with its observed state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// i18n key for the check name.
    pub name_key: String,
    pub state: HealthState,
    /// What was actually observed, e.g. "Service state: Running".
    pub detail: String,
    /// Suggested action, empty when nothing needs doing.
    pub recommendation: String,
}

impl HealthCheck {
    pub fn new(name_key: &str, state: HealthState, detail: &str) -> Self {
        Self {
            name_key: name_key.to_string(),
            state,
            detail: detail.to_string(),
            recommendation: String::new(),
        }
    }

    pub fn with_recommendation(mut self, rec: &str) -> Self {
        self.recommendation = rec.to_string();
        self
    }
}

/// Full health report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub overall: HealthState,
    pub checks: Vec<HealthCheck>,
    /// Output of the most recent SFC run, when one has been run this session.
    pub sfc_outcome: Option<String>,
    pub pending_reboot: bool,
}

impl Default for HealthReport {
    fn default() -> Self {
        Self {
            overall: HealthState::Unknown,
            checks: Vec::new(),
            sfc_outcome: None,
            pending_reboot: false,
        }
    }
}

impl HealthReport {
    /// Checks that are not healthy, most severe first.
    pub fn problems(&self) -> Vec<&HealthCheck> {
        let mut out: Vec<&HealthCheck> = self
            .checks
            .iter()
            .filter(|c| c.state != HealthState::Healthy)
            .collect();
        out.sort_by_key(|a| std::cmp::Reverse(a.state.severity_rank()));
        out
    }

    /// Count of checks in a given state.
    pub fn count(&self, state: HealthState) -> usize {
        self.checks.iter().filter(|c| c.state == state).count()
    }

    /// Whether any check could not be queried. Surfaced so the UI can explain
    /// why the overall verdict is not definitive.
    pub fn has_unknowns(&self) -> bool {
        self.checks.iter().any(|c| c.state == HealthState::Unknown)
    }
}

/// Roll individual checks into one verdict.
///
/// The worst check wins, except that a report with no real problems but some
/// unknowns reports `Unknown` rather than `Healthy`.
pub fn overall_state(checks: &[HealthCheck]) -> HealthState {
    if checks.is_empty() {
        return HealthState::Unknown;
    }
    let worst = checks
        .iter()
        .max_by_key(|c| c.state.severity_rank())
        .map(|c| c.state)
        .unwrap_or(HealthState::Unknown);

    if worst == HealthState::Healthy && checks.iter().any(|c| c.state == HealthState::Unknown) {
        HealthState::Unknown
    } else {
        worst
    }
}

/// Classify a Windows service's state for a health check.
///
/// `state` is the raw service state string from the SCM.
pub fn classify_service_state(state: Option<&str>) -> HealthState {
    match state {
        Some("Running") => HealthState::Healthy,
        Some("Stopped") => HealthState::Critical,
        // Paused / Starting / Stopping are transient or degraded.
        Some("Paused") => HealthState::Warning,
        Some("Starting") | Some("Stopping") => HealthState::Attention,
        Some(_) => HealthState::Unknown,
        None => HealthState::Unknown,
    }
}

/// Classify free space on the system drive.
pub fn classify_disk_free(free_bytes: u64, total_bytes: u64) -> HealthState {
    if total_bytes == 0 {
        return HealthState::Unknown;
    }
    let pct = free_bytes as f64 / total_bytes as f64 * 100.0;
    if pct < 5.0 {
        HealthState::Critical
    } else if pct < 10.0 {
        HealthState::Warning
    } else if pct < 15.0 {
        HealthState::Attention
    } else {
        HealthState::Healthy
    }
}

/// Run a health scan.
///
/// `include_dism` adds the component-store check. It is off by default because
/// DISM's scan is slow; the user can request it explicitly.
pub fn run_health_scan(include_dism: bool) -> HealthReport {
    let mut checks = Vec::new();

    // ---- Windows Update service ----
    checks.push(service_check(
        "healthcenter.windows_update",
        "wuauserv",
        "Windows Update must be able to download and install security fixes.",
    ));

    // ---- Defender ----
    let security = check_security_health();
    checks.push(
        if security.defender_enabled && security.real_time_protection {
            HealthCheck::new(
                "healthcenter.defender",
                HealthState::Healthy,
                "Real-time protection reported enabled.",
            )
        } else if security.defender_enabled {
            HealthCheck::new(
                "healthcenter.defender",
                HealthState::Warning,
                "Antivirus enabled but real-time protection reported off.",
            )
            .with_recommendation("Review real-time protection in Windows Security.")
        } else {
            HealthCheck::new(
                "healthcenter.defender",
                HealthState::Critical,
                "Antivirus reported disabled.",
            )
            .with_recommendation("Enable Microsoft Defender Antivirus in Windows Security.")
        },
    );

    // ---- Firewall ----
    checks.push(if security.firewall_enabled {
        HealthCheck::new(
            "healthcenter.firewall",
            HealthState::Healthy,
            "Firewall profile reported enabled.",
        )
    } else {
        HealthCheck::new(
            "healthcenter.firewall",
            HealthState::Warning,
            "Firewall reported disabled for the standard profile.",
        )
        .with_recommendation("Enable Windows Firewall for the active network profile.")
    });

    // ---- Base Filtering Engine ----
    checks.push(service_check(
        "healthcenter.bfe",
        "BFE",
        "The Base Filtering Engine backs Windows Firewall and IPsec.",
    ));

    // ---- RPC ----
    checks.push(service_check(
        "healthcenter.rpc",
        "RpcSs",
        "Remote Procedure Call is a hard dependency of most Windows components.",
    ));

    // ---- Pending reboot ----
    let pending_reboot = crate::health::update::check_update_health().pending_reboot;
    checks.push(if pending_reboot {
        HealthCheck::new(
            "healthcenter.pending_reboot",
            HealthState::Attention,
            "A restart is pending to finish an update.",
        )
        .with_recommendation("Restart the computer when convenient.")
    } else {
        HealthCheck::new(
            "healthcenter.pending_reboot",
            HealthState::Healthy,
            "No restart is pending.",
        )
    });

    // ---- System drive ----
    let disk = get_primary_disk_stats();
    let disk_state = classify_disk_free(disk.free_bytes, disk.total_bytes);
    let free_gb = disk.free_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    let total_gb = disk.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    checks.push(HealthCheck::new(
        "healthcenter.system_drive",
        disk_state,
        &format!("{:.1} GB free of {:.1} GB.", free_gb, total_gb),
    ));

    // ---- Component store (optional, slow) ----
    if include_dism {
        match crate::health::integrity::run_dism(crate::health::integrity::DismMode::CheckHealth) {
            Ok(output) => {
                let lower = output.to_lowercase();
                let state = if lower.contains("no component store corruption detected") {
                    HealthState::Healthy
                } else if lower.contains("repairable") {
                    HealthState::Warning
                } else {
                    // Repairable corruption is the only other definite verdict
                    // DISM reports; anything else stays unknown.
                    HealthState::Unknown
                };
                checks.push(HealthCheck::new(
                    "healthcenter.component_store",
                    state,
                    "DISM component store check completed.",
                ));
            }
            Err(e) => {
                checks.push(HealthCheck::new(
                    "healthcenter.component_store",
                    HealthState::Unknown,
                    &format!("DISM check could not be completed: {}", e),
                ));
            }
        }
    } else {
        checks.push(HealthCheck::new(
            "healthcenter.component_store",
            HealthState::Unknown,
            "Not checked. Run DISM CheckHealth to include it.",
        ));
    }

    // ---- System file integrity (only known once SFC has been run) ----
    checks.push(HealthCheck::new(
        "healthcenter.file_integrity",
        HealthState::Unknown,
        "Integrity has not been checked in this session.",
    ));

    let overall = overall_state(&checks);

    HealthReport {
        overall,
        checks,
        sfc_outcome: None,
        pending_reboot,
    }
}

/// Build a health check from a live service query.
fn service_check(name_key: &str, service_name: &str, why: &str) -> HealthCheck {
    let state = service_state_query(service_name);
    let health = classify_service_state(state.as_deref());
    let detail = match &state {
        Some(s) => format!("Service state: {}. {}", s, why),
        None => format!("Service '{}' could not be queried.", service_name),
    };

    let check = HealthCheck::new(name_key, health, &detail);
    match health {
        HealthState::Critical => check.with_recommendation(&format!(
            "Start the '{}' service. It is required by Windows.",
            service_name
        )),
        HealthState::Unknown => check.with_recommendation(
            "Query the service state manually with Get-Service in an elevated PowerShell.",
        ),
        _ => check,
    }
}

/// Fold an SFC run into a report, replacing the integrity check.
pub fn apply_sfc_result(report: &mut HealthReport, output: &str) {
    let outcome = interpret_sfc_output(output);
    report.sfc_outcome = Some(
        match outcome {
            IntegrityOutcome::Clean => "clean",
            IntegrityOutcome::Repaired => "repaired",
            IntegrityOutcome::Unknown => "unknown",
        }
        .to_string(),
    );

    let state = match outcome {
        IntegrityOutcome::Clean => HealthState::Healthy,
        IntegrityOutcome::Repaired => HealthState::Warning,
        IntegrityOutcome::Unknown => HealthState::Unknown,
    };
    let detail = match outcome {
        IntegrityOutcome::Clean => "SFC reported no integrity violations.",
        IntegrityOutcome::Repaired => {
            "SFC reported corrupt files and attempted repair. Review its output."
        }
        IntegrityOutcome::Unknown => "SFC output could not be interpreted.",
    };

    if let Some(check) = report
        .checks
        .iter_mut()
        .find(|c| c.name_key == "healthcenter.file_integrity")
    {
        check.state = state;
        check.detail = detail.to_string();
    } else {
        report.checks.push(HealthCheck::new(
            "healthcenter.file_integrity",
            state,
            detail,
        ));
    }

    report.overall = overall_state(&report.checks);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overall_is_worst_check_wins() {
        let checks = vec![
            HealthCheck::new("a", HealthState::Healthy, ""),
            HealthCheck::new("b", HealthState::Attention, ""),
            HealthCheck::new("c", HealthState::Warning, ""),
        ];
        assert_eq!(overall_state(&checks), HealthState::Warning);

        let critical = vec![
            HealthCheck::new("a", HealthState::Warning, ""),
            HealthCheck::new("b", HealthState::Critical, ""),
        ];
        assert_eq!(overall_state(&critical), HealthState::Critical);
    }

    #[test]
    fn unknown_checks_do_not_produce_a_healthy_overall() {
        let checks = vec![
            HealthCheck::new("a", HealthState::Healthy, ""),
            HealthCheck::new("b", HealthState::Unknown, ""),
        ];
        assert_eq!(overall_state(&checks), HealthState::Unknown);
    }

    #[test]
    fn all_healthy_checks_produce_healthy_overall() {
        let checks = vec![
            HealthCheck::new("a", HealthState::Healthy, ""),
            HealthCheck::new("b", HealthState::Healthy, ""),
        ];
        assert_eq!(overall_state(&checks), HealthState::Healthy);
    }

    #[test]
    fn empty_report_is_unknown_not_healthy() {
        assert_eq!(overall_state(&[]), HealthState::Unknown);
    }

    #[test]
    fn service_state_classification_is_explicit_about_unknown() {
        assert_eq!(
            classify_service_state(Some("Running")),
            HealthState::Healthy
        );
        assert_eq!(
            classify_service_state(Some("Stopped")),
            HealthState::Critical
        );
        assert_eq!(classify_service_state(Some("Paused")), HealthState::Warning);
        assert_eq!(
            classify_service_state(Some("Starting")),
            HealthState::Attention
        );
        assert_eq!(classify_service_state(Some("Weird")), HealthState::Unknown);
        assert_eq!(classify_service_state(None), HealthState::Unknown);
    }

    #[test]
    fn disk_free_thresholds_escalate_as_space_runs_out() {
        let total = 1000u64;
        assert_eq!(classify_disk_free(500, total), HealthState::Healthy);
        assert_eq!(classify_disk_free(120, total), HealthState::Attention);
        assert_eq!(classify_disk_free(80, total), HealthState::Warning);
        assert_eq!(classify_disk_free(20, total), HealthState::Critical);
        // Unknown total must not be reported as healthy.
        assert_eq!(classify_disk_free(0, 0), HealthState::Unknown);
    }

    #[test]
    fn sfc_result_replaces_the_integrity_check() {
        let mut report = HealthReport {
            overall: HealthState::Unknown,
            checks: vec![HealthCheck::new(
                "healthcenter.file_integrity",
                HealthState::Unknown,
                "not checked",
            )],
            sfc_outcome: None,
            pending_reboot: false,
        };

        apply_sfc_result(
            &mut report,
            "Windows Resource Protection did not find any integrity violations.",
        );
        assert_eq!(report.sfc_outcome.as_deref(), Some("clean"));
        let integrity = report
            .checks
            .iter()
            .find(|c| c.name_key == "healthcenter.file_integrity")
            .expect("integrity check present");
        assert_eq!(integrity.state, HealthState::Healthy);
        assert_eq!(report.overall, HealthState::Healthy);
    }

    #[test]
    fn uninterpretable_sfc_output_stays_unknown() {
        let mut report = HealthReport::default();
        apply_sfc_result(&mut report, "Beginning system scan.");
        assert_eq!(report.sfc_outcome.as_deref(), Some("unknown"));
        assert_eq!(report.overall, HealthState::Unknown);
        assert!(report.has_unknowns());
    }

    #[test]
    fn problems_lists_non_healthy_checks_most_severe_first() {
        let report = HealthReport {
            overall: HealthState::Critical,
            checks: vec![
                HealthCheck::new("ok", HealthState::Healthy, ""),
                HealthCheck::new("warn", HealthState::Warning, ""),
                HealthCheck::new("crit", HealthState::Critical, ""),
                HealthCheck::new("unk", HealthState::Unknown, ""),
            ],
            sfc_outcome: None,
            pending_reboot: false,
        };
        let problems = report.problems();
        assert_eq!(problems.len(), 3);
        assert_eq!(problems[0].name_key, "crit");
        assert_eq!(problems[1].name_key, "warn");
        assert_eq!(problems[2].name_key, "unk");
        assert_eq!(report.count(HealthState::Healthy), 1);
    }
}
