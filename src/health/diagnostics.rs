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
            SystemHealthRating::Excellent => (34, 197, 94),   // Green
            SystemHealthRating::Good => (16, 185, 129),       // Emerald
            SystemHealthRating::Attention => (234, 179, 8),   // Yellow
            SystemHealthRating::Warning => (249, 115, 22),    // Orange
            SystemHealthRating::Critical => (239, 68, 68),   // Red
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealthReport {
    pub rating: SystemHealthRating,
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

    if !sec.defender_enabled {
        issues.push("Real-time antivirus protection is turned off.".to_string());
        recommendations.push("Enable Microsoft Defender Antivirus in Windows Security.".to_string());
    }

    if upd.pending_reboot {
        issues.push("System restart pending for Windows Update installation.".to_string());
        recommendations.push("Restart the computer to complete updates.".to_string());
    }

    if disk.total_bytes > 0 && (disk.free_bytes as f32 / disk.total_bytes as f32) < 0.10 {
        issues.push(format!("Primary system drive (C:) is running low on space ({:.1}% free).", 100.0 - disk.usage_pct));
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
            recommendations.push("Memory usage is moderate. Background trimming optional.".to_string());
        }
        MemoryPressure::Low => {}
    }

    let rating = if mem.pressure == MemoryPressure::Critical || !sec.defender_enabled {
        SystemHealthRating::Warning
    } else if mem.pressure == MemoryPressure::High || upd.pending_reboot {
        SystemHealthRating::Attention
    } else if !issues.is_empty() || mem.pressure == MemoryPressure::Moderate {
        SystemHealthRating::Good
    } else {
        SystemHealthRating::Excellent
    };

    if recommendations.is_empty() {
        recommendations.push("No critical system changes required. Your system is healthy.".to_string());
    }

    SystemHealthReport {
        rating,
        issues,
        recommendations,
    }
}
