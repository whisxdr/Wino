use crate::monitoring::ram::RamStats;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryPressure {
    Low,
    Moderate,
    High,
    Critical,
}

impl MemoryPressure {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryPressure::Low => "LOW",
            MemoryPressure::Moderate => "MODERATE",
            MemoryPressure::High => "HIGH",
            MemoryPressure::Critical => "CRITICAL",
        }
    }

    pub fn color_rgb(&self) -> (u8, u8, u8) {
        match self {
            MemoryPressure::Low => (34, 197, 94),     // Green
            MemoryPressure::Moderate => (234, 179, 8), // Yellow
            MemoryPressure::High => (249, 115, 22),    // Orange
            MemoryPressure::Critical => (239, 68, 68), // Red
        }
    }
}

pub fn calculate_memory_pressure(stats: &RamStats) -> MemoryPressure {
    let available_gb = stats.available_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    let commit_ratio = if stats.commit_limit_bytes > 0 {
        stats.commit_used_bytes as f32 / stats.commit_limit_bytes as f32
    } else {
        0.0
    };

    if stats.usage_pct >= 92.0 && available_gb < 1.0 {
        MemoryPressure::Critical
    } else if (stats.usage_pct >= 85.0 && available_gb < 2.0) || commit_ratio >= 0.90 {
        MemoryPressure::High
    } else if stats.usage_pct >= 70.0 || commit_ratio >= 0.75 {
        MemoryPressure::Moderate
    } else {
        MemoryPressure::Low
    }
}
