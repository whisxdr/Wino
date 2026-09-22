use crate::health::defender::check_security_health;
use crate::health::update::check_update_health;
use crate::memory::monitor::capture_memory_snapshot;
use crate::memory::pressure::MemoryPressure;
use crate::monitoring::disk::get_primary_disk_stats;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemHealthRating {
    Excellent,
    Good,
    Attention,
    Warning,
    Critical,
}

impl SystemHealthRating {
    pub fn as_str(&self) -> &'static str {
        match self {
            SystemHealthRating::Excellent => "EXCELLENT",
            SystemHealthRating::Good => "GOOD",
            SystemHealthRating::Attention => "ATTENTION",
            SystemHealthRating::Warning => "WARNING",
            SystemHealthRating::Critical => "CRITICAL",
        }
    }

    pub fn color_rgb(&self) -> (u8, u8, u8) {
        match self {
            SystemHealthRating::Excellent => (34, 197, 94), // Green
            SystemHealthRating::Good => (16, 185, 129),     // Emerald
            SystemHealthRating::Attention => (234, 179, 8), // Yellow
            SystemHealthRating::Warning => (249, 115, 22),  // Orange
            SystemHealthRating::Critical => (239, 68, 68),  // Red
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealthReport {
    pub rating: SystemHealthRating,
    pub defender_active: bool,
    pub firewall_active: bool,
    pub pending_reboot: bool,
    pub issues: Vec<String>,
    pub recommendations: Vec<String>,
}

pub fn evaluate_system_health() -> SystemHealthReport {
    let mem = capture_memory_snapshot();
    let disk = get_primary_disk_stats();
    let sec = check_security_health();
    let upd = check_update_health();

    let mut issues = Vec::new();
    let mut recommendations = Vec::new();

    let defender_active = sec.defender_enabled;
    let firewall_active = sec.firewall_enabled;
    let pending_reboot = upd.pending_reboot;

    if !defender_active {
        issues.push("Real-time antivirus protection is turned off.".to_string());
        recommendations
            .push("Enable Microsoft Defender Antivirus in Windows Security.".to_string());
    }

    if pending_reboot {
        issues.push("System restart pending for Windows Update installation.".to_string());
        recommendations.push("Restart the computer to complete updates.".to_string());
    }

    if disk.total_bytes > 0 && (disk.free_bytes as f32 / disk.total_bytes as f32) < 0.10 {
        issues.push(format!(
            "Primary system drive (C:) is running low on space ({:.1}% free).",
            100.0 - disk.usage_pct
        ));
        recommendations.push("Run Storage Cleaner to remove temporary files.".to_string());
    }

    match mem.pressure {
        MemoryPressure::Critical => {
            issues.push("Critical memory pressure detected (available RAM < 1 GB).".to_string());
            recommendations.push("Perform Smart Memory Optimization.".to_string());
        }
        MemoryPressure::High => {
            issues.push("High memory utilization detected.".to_string());
            recommendations.push("Trim idle background process working sets.".to_string());
        }
        MemoryPressure::Moderate => {
            recommendations
                .push("Memory usage is moderate. Background trimming optional.".to_string());
        }
        MemoryPressure::Low => {}
    }

    let rating = if mem.pressure == MemoryPressure::Critical || !defender_active {
        SystemHealthRating::Critical
    } else if mem.pressure == MemoryPressure::High || pending_reboot {
        SystemHealthRating::Warning
    } else if !issues.is_empty() {
        SystemHealthRating::Attention
    } else if mem.pressure == MemoryPressure::Moderate {
        SystemHealthRating::Good
    } else {
        SystemHealthRating::Excellent
    };

    SystemHealthReport {
        rating,
        defender_active,
        firewall_active,
        pending_reboot,
        issues,
        recommendations,
    }
}
