use serde::{Deserialize, Serialize};

/// Risk category of a scheduled task targeted for debloating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskCategory {
    /// Microsoft telemetry / diagnostics / CEIP
    Telemetry,
    /// Microsoft feedback & experience programs
    Feedback,
    /// Third-party auto-updaters and vendor nagware
    ThirdPartyUpdate,
}

impl TaskCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskCategory::Telemetry => "Telemetry",
            TaskCategory::Feedback => "Feedback / CEIP",
            TaskCategory::ThirdPartyUpdate => "Third-Party Updater",
        }
    }

    /// Whether disabling this category is considered low-risk & reversible.
    pub fn is_safe_to_disable(&self) -> bool {
        matches!(
            self,
            Self::Telemetry | Self::Feedback | Self::ThirdPartyUpdate
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTaskRule {
    /// Full task path as used by schtasks.exe (/TN), e.g.
    /// "\Microsoft\Windows\Application Experience\Microsoft Compatibility Appraiser"
    pub task_path: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub category: TaskCategory,
}

/// Curated hidden background tasks that burn CPU cycles collecting
/// telemetry, compatibility data, or phoning home. All are safe to disable:
/// Windows can bring them back after a feature update when it needs them.
pub const TASK_RULES: &[ScheduledTaskRule] = &[
    ScheduledTaskRule {
        task_path: "\\Microsoft\\Windows\\Application Experience\\Microsoft Compatibility Appraiser",
        name: "Microsoft Compatibility Appraiser",
        description: "Scans drives and executables for compatibility data. It spikes CPU and disk while you idle.",
        category: TaskCategory::Telemetry,
    },
    ScheduledTaskRule {
        task_path: "\\Microsoft\\Windows\\Application Experience\\ProgramDataUpdater",
        name: "Program Data Updater",
        description: "Uploads collected application-telemetry data to Microsoft servers.",
        category: TaskCategory::Telemetry,
    },
    ScheduledTaskRule {
        task_path: "\\Microsoft\\Windows\\Application Experience\\PcaPatchDbTask",
        name: "PCA Patch Database Task",
        description: "Updates the Program Compatibility Assistant patch database.",
        category: TaskCategory::Telemetry,
    },
    ScheduledTaskRule {
        task_path: "\\Microsoft\\Windows\\Application Experience\\MareBackup",
        name: "Mare Backup",
        description: "Backs up Application Experience telemetry state.",
        category: TaskCategory::Telemetry,
    },
    ScheduledTaskRule {
        task_path: "\\Microsoft\\Windows\\Application Experience\\StartupAppTask",
        name: "Startup App Task",
        description: "Monitors startup apps; disabling removes startup telemetry polling.",
        category: TaskCategory::Telemetry,
    },
    ScheduledTaskRule {
        task_path: "\\Microsoft\\Windows\\Customer Experience Improvement Program\\Consolidator",
        name: "CEIP Consolidator (SQM)",
        description: "Consolidates Customer Experience Improvement Program usage data before upload.",
        category: TaskCategory::Feedback,
    },
    ScheduledTaskRule {
        task_path: "\\Microsoft\\Windows\\Customer Experience Improvement Program\\UsbCeip",
        name: "CEIP USB Session",
        description: "Collects USB device usage data for the Customer Experience Improvement Program.",
        category: TaskCategory::Feedback,
    },
    ScheduledTaskRule {
        task_path: "\\Microsoft\\Windows\\Customer Experience Improvement Program\\Uploader",
        name: "CEIP Uploader",
        description: "Uploads queued CEIP/SQM payloads to Microsoft.",
        category: TaskCategory::Feedback,
    },
    ScheduledTaskRule {
        task_path: "\\Microsoft\\Windows\\Windows Error Reporting\\QueueReporting",
        name: "Error Reporting Queue",
        description: "Queues and uploads crash/error reports to Microsoft WER servers.",
        category: TaskCategory::Telemetry,
    },
    ScheduledTaskRule {
        task_path: "\\Microsoft\\Windows\\Feedback\\Siuf\\DmClient",
        name: "Feedback Siuf DmClient",
        description: "Device-management client for the Windows Feedback platform.",
        category: TaskCategory::Feedback,
    },
    ScheduledTaskRule {
        task_path: "\\Microsoft\\Windows\\Feedback\\Siuf\\DmClientOnScenarioDownload",
        name: "Feedback Siuf Download Client",
        description: "Downloads feedback-scenario content in the background.",
        category: TaskCategory::Feedback,
    },
    ScheduledTaskRule {
        task_path: "\\Microsoft\\Windows\\Autochk\\Proxy",
        name: "Autochk Proxy",
        description: "Reports disk-check results to Microsoft telemetry.",
        category: TaskCategory::Telemetry,
    },
    ScheduledTaskRule {
        task_path: "\\Microsoft\\Windows\\DiskDiagnostic\\Microsoft-Windows-DiskDiagnosticDataCollector",
        name: "Disk Diagnostic Data Collector",
        description: "Collects disk diagnostic data for OEM/Microsoft analytics.",
        category: TaskCategory::Telemetry,
    },
    ScheduledTaskRule {
        task_path: "\\Microsoft\\Windows\\Maps\\MapsUpdateTask",
        name: "Maps Update Task",
        description: "Downloads map updates in the background while Maps stays closed.",
        category: TaskCategory::Telemetry,
    },
    ScheduledTaskRule {
        task_path: "\\Microsoft\\Windows\\Maps\\MapsToastTask",
        name: "Maps Toast Task",
        description: "Pops promotional Maps notifications from the background.",
        category: TaskCategory::Feedback,
    },
    ScheduledTaskRule {
        task_path: "\\Microsoft\\Windows\\Device Information\\Device",
        name: "Device Information Collector",
        description: "Gathers hardware/device information for Microsoft services.",
        category: TaskCategory::Telemetry,
    },
];

/// Filename fragments used to detect third-party vendor updaters that are not
/// covered by the curated list above (matched against the on-disk task file).
pub const THIRD_PARTY_HEURISTICS: &[&str] = &[
    // Adobe
    "adobe",
    "acrobat",
    "arm.exe",
    // Google
    "googleupdate",
    // Browsers / utilities
    "opera_autoupdate",
    "brave",
    "vivaldi",
    "edgeupdate",
    "microsooftedgeupdate",
    // Vendor nagware / helpers
    "nvidia web driver",
    "geforce experience",
    "onedrivestandaloneupdate",
    "hp support",
    "lenovo",
    "dell update",
    "supportassist",
    "asus update",
    "spotifyautostart",
    "steamwebhelper_",
    "discordupdate",
];

/// Pure helper: does an on-disk task path look like a third-party updater?
pub fn matches_third_party_heuristic(path_lower: &str) -> bool {
    THIRD_PARTY_HEURISTICS
        .iter()
        .any(|frag| path_lower.contains(frag))
}

/// Pure helper: find the matching curated rule for a task path (case-insensitive).
pub fn find_rule(task_path_lower: &str) -> Option<&'static ScheduledTaskRule> {
    TASK_RULES
        .iter()
        .find(|r| r.task_path.to_lowercase() == task_path_lower)
}
