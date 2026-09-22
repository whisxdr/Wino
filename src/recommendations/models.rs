//! System-level recommendation model.
//!
//! A recommendation is an *observation*, never an action. Every field is either
//! a measured value or a pointer at the view where the user can review and
//! decide. Nothing here can be applied without the user opening that view.

use serde::{Deserialize, Serialize};

/// Which subsystem the recommendation concerns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RecommendationArea {
    Memory,
    Storage,
    Startup,
    Services,
    Privacy,
    Power,
    Security,
    Network,
    Updates,
    Cleanup,
}

impl RecommendationArea {
    pub fn i18n_key(&self) -> &'static str {
        match self {
            RecommendationArea::Memory => "rec.area_memory",
            RecommendationArea::Storage => "rec.area_storage",
            RecommendationArea::Startup => "rec.area_startup",
            RecommendationArea::Services => "rec.area_services",
            RecommendationArea::Privacy => "rec.area_privacy",
            RecommendationArea::Power => "rec.area_power",
            RecommendationArea::Security => "rec.area_security",
            RecommendationArea::Network => "rec.area_network",
            RecommendationArea::Updates => "rec.area_updates",
            RecommendationArea::Cleanup => "rec.area_cleanup",
        }
    }

    pub fn color_rgb(&self) -> (u8, u8, u8) {
        match self {
            RecommendationArea::Memory => (60, 144, 255),
            RecommendationArea::Storage => (167, 139, 250),
            RecommendationArea::Startup => (255, 185, 95),
            RecommendationArea::Services => (78, 222, 163),
            RecommendationArea::Privacy => (236, 72, 153),
            RecommendationArea::Power => (250, 204, 21),
            RecommendationArea::Security => (239, 68, 68),
            RecommendationArea::Network => (56, 189, 248),
            RecommendationArea::Updates => (16, 185, 129),
            RecommendationArea::Cleanup => (148, 163, 184),
        }
    }
}

/// How much attention the observation deserves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RecommendationSeverity {
    Info,
    Low,
    Medium,
    High,
}

impl RecommendationSeverity {
    pub fn i18n_key(&self) -> &'static str {
        match self {
            RecommendationSeverity::High => "rec.severity_high",
            RecommendationSeverity::Medium => "rec.severity_medium",
            RecommendationSeverity::Low => "rec.severity_low",
            RecommendationSeverity::Info => "rec.severity_info",
        }
    }

    pub fn color_rgb(&self) -> (u8, u8, u8) {
        match self {
            RecommendationSeverity::High => (239, 68, 68),
            RecommendationSeverity::Medium => (234, 179, 8),
            RecommendationSeverity::Low => (56, 189, 248),
            RecommendationSeverity::Info => (148, 163, 184),
        }
    }
}

/// Which view the "Review" button opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewTarget {
    Memory,
    Processes,
    Cleaner,
    Storage,
    Startup,
    Services,
    Privacy,
    Power,
    Security,
    Network,
    Apps,
    Health,
    Recommendations,
}

/// One measured observation with a place to act on it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    /// Stable rule id, so the same observation is not duplicated per scan.
    pub id: String,
    pub area: RecommendationArea,
    pub severity: RecommendationSeverity,
    /// Headline, already localized at build time from a dictionary key.
    pub title: String,
    /// The measured numbers behind the headline, e.g. "5.4 GB of 16 GB (34%)".
    pub measured: String,
    /// Where the user reviews and decides.
    pub target: ReviewTarget,
}

impl Recommendation {
    pub fn new(
        id: &str,
        area: RecommendationArea,
        severity: RecommendationSeverity,
        title: &str,
        measured: &str,
        target: ReviewTarget,
    ) -> Self {
        Self {
            id: id.to_string(),
            area,
            severity,
            title: title.to_string(),
            measured: measured.to_string(),
            target,
        }
    }
}

/// Sort recommendations so the most attention-worthy come first, then by area
/// so the list stays stable between scans.
pub fn sort_recommendations(items: &mut [Recommendation]) {
    items.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then_with(|| format!("{:?}", a.area).cmp(&format!("{:?}", b.area)))
            .then_with(|| a.id.cmp(&b.id))
    });
}

/// Format a byte count as GB with one decimal, for measured-value strings.
pub fn gb(bytes: u64) -> String {
    format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorting_puts_high_severity_first() {
        let mut items = vec![
            Recommendation::new(
                "b",
                RecommendationArea::Memory,
                RecommendationSeverity::Low,
                "low",
                "",
                ReviewTarget::Memory,
            ),
            Recommendation::new(
                "a",
                RecommendationArea::Startup,
                RecommendationSeverity::High,
                "high",
                "",
                ReviewTarget::Startup,
            ),
            Recommendation::new(
                "c",
                RecommendationArea::Storage,
                RecommendationSeverity::Medium,
                "med",
                "",
                ReviewTarget::Storage,
            ),
        ];
        sort_recommendations(&mut items);
        assert_eq!(items[0].id, "a");
        assert_eq!(items[1].id, "c");
        assert_eq!(items[2].id, "b");
    }

    #[test]
    fn gb_formats_binary_gigabytes() {
        assert_eq!(gb(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(gb(5 * 1024 * 1024 * 1024), "5.0 GB");
        assert_eq!(gb(0), "0.0 GB");
    }

    #[test]
    fn severity_ordering_is_info_low_medium_high() {
        assert!(RecommendationSeverity::High > RecommendationSeverity::Medium);
        assert!(RecommendationSeverity::Medium > RecommendationSeverity::Low);
        assert!(RecommendationSeverity::Low > RecommendationSeverity::Info);
    }
}
