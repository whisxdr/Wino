//! Storage Analyzer data model.
//!
//! The analyzer is read-only by construction: nothing in this module can delete
//! a file. It exists to answer "where did the space go", and every reclaim path
//! goes through the existing Storage Cleaner and its safety workflow.

use serde::{Deserialize, Serialize};

/// Top-level bucket a path is classified into, so the root of a drive shows a
/// short, stable summary instead of thousands of sibling folders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StorageBucket {
    Applications,
    Windows,
    Users,
    ProgramData,
    Temp,
    Other,
}

impl StorageBucket {
    pub fn i18n_key(&self) -> &'static str {
        match self {
            StorageBucket::Applications => "storage.bucket_applications",
            StorageBucket::Windows => "storage.bucket_windows",
            StorageBucket::Users => "storage.bucket_users",
            StorageBucket::ProgramData => "storage.bucket_programdata",
            StorageBucket::Temp => "storage.bucket_temp",
            StorageBucket::Other => "storage.bucket_other",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            StorageBucket::Applications => "Applications",
            StorageBucket::Windows => "Windows",
            StorageBucket::Users => "Users",
            StorageBucket::ProgramData => "ProgramData",
            StorageBucket::Temp => "Temp",
            StorageBucket::Other => "Other",
        }
    }

    pub fn color_rgb(&self) -> (u8, u8, u8) {
        match self {
            StorageBucket::Applications => (60, 144, 255),
            StorageBucket::Windows => (56, 189, 248),
            StorageBucket::Users => (78, 222, 163),
            StorageBucket::ProgramData => (167, 139, 250),
            StorageBucket::Temp => (255, 185, 95),
            StorageBucket::Other => (148, 163, 184),
        }
    }

    /// Buckets in display order.
    pub const ALL: [StorageBucket; 6] = [
        StorageBucket::Applications,
        StorageBucket::Windows,
        StorageBucket::Users,
        StorageBucket::ProgramData,
        StorageBucket::Temp,
        StorageBucket::Other,
    ];
}

/// One child entry of a scanned directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    /// Total bytes. For a directory this is the recursive sum of the files
    /// visited inside it, which is a lower bound when the scan hit a limit.
    pub size_bytes: u64,
    /// Number of files counted under this entry.
    pub file_count: usize,
    /// True when the entry was not fully measured (scan limit, cancellation,
    /// or access denied).
    pub partial: bool,
    pub bucket: StorageBucket,
}

impl StorageEntry {
    /// Share of the parent total, 0.0 when the parent total is unknown.
    pub fn fraction_of(&self, parent_total: u64) -> f32 {
        if parent_total == 0 {
            return 0.0;
        }
        (self.size_bytes as f64 / parent_total as f64) as f32
    }
}

/// A file that crossed the configured size threshold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LargeFile {
    pub path: String,
    pub size_bytes: u64,
    /// Days since last modification, `None` when the timestamp was unreadable.
    pub age_days: Option<u64>,
}

/// A directory that could be routed into the Storage Cleaner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheLocation {
    /// Matches a `data/cleanup_rules.json` rule id when one covers this path.
    pub rule_id: String,
    pub label: String,
    pub path: String,
    pub size_bytes: u64,
    pub file_count: usize,
}

/// Result of one storage scan.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorageTree {
    /// Drive root that was scanned, e.g. `C:\`.
    pub root: String,
    /// Directory the user is currently viewing.
    pub current_path: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub used_bytes: u64,
    /// Children of `current_path`, largest first.
    pub entries: Vec<StorageEntry>,
    /// Per-bucket totals for the whole scan, in [`StorageBucket::ALL`] order.
    pub bucket_totals: Vec<(StorageBucket, u64)>,
    pub large_files: Vec<LargeFile>,
    pub old_files: Vec<LargeFile>,
    pub cache_locations: Vec<CacheLocation>,
    pub files_scanned: usize,
    pub dirs_scanned: usize,
    /// Directories skipped because access was denied.
    pub skipped_inaccessible: usize,
    /// Reparse points (symlinks, junctions, mount points) not followed.
    pub skipped_reparse: usize,
    /// True when the entry budget was exhausted before the tree was complete.
    pub limit_reached: bool,
    /// True when the user cancelled the scan.
    pub cancelled: bool,
}

impl StorageTree {
    /// Total for one bucket.
    pub fn bucket_total(&self, bucket: StorageBucket) -> u64 {
        self.bucket_totals
            .iter()
            .find(|(b, _)| *b == bucket)
            .map(|(_, total)| *total)
            .unwrap_or(0)
    }

    /// Used fraction of the drive, 0.0 when the total is unknown.
    pub fn used_fraction(&self) -> f32 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        (self.used_bytes as f64 / self.total_bytes as f64) as f32
    }

    /// True when the scan was cut short for any reason, so the UI can say so
    /// instead of presenting a partial tree as complete.
    pub fn is_partial(&self) -> bool {
        self.limit_reached || self.cancelled || self.skipped_inaccessible > 0
    }
}

/// Classify an absolute path into a [`StorageBucket`].
///
/// Pure, so the classification rules are testable without touching a disk.
/// Matching is case-insensitive and anchored on whole path components, so
/// `C:\WindowsApps` is not mistaken for `C:\Windows`.
pub fn classify_path(path: &str) -> StorageBucket {
    let lower = path.to_lowercase().replace('/', "\\");
    let trimmed = lower.trim_end_matches('\\');

    let has_component = |component: &str| trimmed.split('\\').any(|part| part == component);

    // Temp first: a Temp folder inside a user profile still belongs to Temp.
    if has_component("temp") || has_component("tmp") {
        return StorageBucket::Temp;
    }
    if has_component("program files") || has_component("program files (x86)") {
        return StorageBucket::Applications;
    }
    if has_component("programdata") {
        return StorageBucket::ProgramData;
    }
    if has_component("windows") {
        return StorageBucket::Windows;
    }
    if has_component("users") {
        return StorageBucket::Users;
    }
    StorageBucket::Other
}

/// Whether a file should be reported as "old" given its age in days.
pub fn is_old_file(age_days: Option<u64>, threshold_days: u64) -> bool {
    match age_days {
        Some(age) => age >= threshold_days,
        None => false,
    }
}

/// Whether a file should be reported as "large" given its size in bytes.
pub fn is_large_file(size_bytes: u64, threshold_mb: u64) -> bool {
    size_bytes >= threshold_mb.saturating_mul(1024 * 1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_path_is_component_anchored() {
        assert_eq!(
            classify_path(r"C:\Windows\System32"),
            StorageBucket::Windows
        );
        // Not Windows: the component is "WindowsApps", not "Windows".
        assert_eq!(classify_path(r"C:\WindowsApps\Thing"), StorageBucket::Other);
        assert_eq!(
            classify_path(r"C:\Program Files\App"),
            StorageBucket::Applications
        );
        assert_eq!(
            classify_path(r"C:\Program Files (x86)\App"),
            StorageBucket::Applications
        );
        assert_eq!(classify_path(r"C:\Users\Ada"), StorageBucket::Users);
        assert_eq!(
            classify_path(r"C:\ProgramData\Vendor"),
            StorageBucket::ProgramData
        );
        assert_eq!(classify_path(r"C:\"), StorageBucket::Other);
    }

    #[test]
    fn classify_path_puts_temp_first_regardless_of_parent() {
        assert_eq!(
            classify_path(r"C:\Users\Ada\AppData\Local\Temp"),
            StorageBucket::Temp
        );
        assert_eq!(classify_path(r"C:\Windows\Temp"), StorageBucket::Temp);
        assert_eq!(classify_path(r"D:\tmp\work"), StorageBucket::Temp);
    }

    #[test]
    fn classify_path_normalizes_separators_and_case() {
        assert_eq!(classify_path("c:/windows/system32"), StorageBucket::Windows);
        assert_eq!(classify_path(r"C:\WINDOWS\"), StorageBucket::Windows);
    }

    #[test]
    fn age_and_size_thresholds_are_inclusive() {
        assert!(is_old_file(Some(365), 365));
        assert!(!is_old_file(Some(364), 365));
        // An unreadable timestamp is never reported as old.
        assert!(!is_old_file(None, 365));

        assert!(is_large_file(100 * 1024 * 1024, 100));
        assert!(!is_large_file(99 * 1024 * 1024, 100));
    }

    #[test]
    fn tree_partial_detection_and_used_fraction() {
        let mut tree = StorageTree {
            total_bytes: 1000,
            used_bytes: 250,
            ..Default::default()
        };
        assert!((tree.used_fraction() - 0.25).abs() < 0.001);
        assert!(!tree.is_partial());

        tree.skipped_inaccessible = 1;
        assert!(tree.is_partial());

        let empty = StorageTree::default();
        assert_eq!(empty.used_fraction(), 0.0);
        assert_eq!(empty.bucket_total(StorageBucket::Windows), 0);
    }

    #[test]
    fn entry_fraction_handles_zero_parent() {
        let entry = StorageEntry {
            name: "x".to_string(),
            path: r"C:\x".to_string(),
            is_dir: true,
            size_bytes: 50,
            file_count: 1,
            partial: false,
            bucket: StorageBucket::Other,
        };
        assert_eq!(entry.fraction_of(0), 0.0);
        assert!((entry.fraction_of(100) - 0.5).abs() < 0.001);
    }
}
