//! Presentation helpers for a [`StorageTree`].
//!
//! These live apart from the scanner because they are pure: the UI can format a
//! cached tree, a drill-down view, or a tree restored from a saved report
//! without touching a disk again. Nothing in this module can read or delete a
//! file, and nothing here blocks, so it is safe to call from the render loop.

use crate::storage::models::{StorageBucket, StorageEntry, StorageTree};

/// Format a byte count with binary units.
///
/// Same convention as `apps::models::format_size` (two decimals for GB, one for
/// MB, none for KB) so a size looks identical wherever Wino prints it. Kept
/// local rather than imported so the storage subsystem stays independent of the
/// application manager.
pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let value = bytes as f64;

    if value >= GB {
        format!("{:.2} GB", value / GB)
    } else if value >= MB {
        format!("{:.1} MB", value / MB)
    } else if value >= KB {
        format!("{:.0} KB", value / KB)
    } else {
        format!("{} B", bytes)
    }
}

/// Per-bucket totals with their share of the scanned bytes, in
/// [`StorageBucket::ALL`] order.
///
/// The denominator is the sum of the bucket totals, not the drive capacity: a
/// scan that stopped at a limit still produces a chart that fills, and the
/// difference against `used_bytes` is what tells the user the scan was partial.
/// Every bucket is returned, including empty ones, so the legend stays stable.
pub fn bucket_percentages(tree: &StorageTree) -> Vec<(StorageBucket, u64, f32)> {
    let scanned_total: u64 = tree.bucket_totals.iter().map(|(_, bytes)| *bytes).sum();

    StorageBucket::ALL
        .iter()
        .map(|bucket| {
            let bytes = tree.bucket_total(*bucket);
            let percent = if scanned_total == 0 {
                0.0
            } else {
                (bytes as f64 / scanned_total as f64 * 100.0) as f32
            };
            (*bucket, bytes, percent)
        })
        .collect()
}

/// The `limit` largest children of the tree's current directory.
///
/// Sorts defensively instead of trusting the input order, so a tree restored
/// from JSON reports the same ranking as a freshly scanned one.
pub fn top_entries(tree: &StorageTree, limit: usize) -> Vec<&StorageEntry> {
    let mut ranked: Vec<&StorageEntry> = tree.entries.iter().collect();
    ranked.sort_by_key(|a| std::cmp::Reverse(a.size_bytes));
    ranked.truncate(limit);
    ranked
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::models::LargeFile;

    fn entry(name: &str, size: u64) -> StorageEntry {
        StorageEntry {
            name: name.to_string(),
            path: format!(r"C:\{}", name),
            is_dir: true,
            size_bytes: size,
            file_count: 1,
            partial: false,
            bucket: StorageBucket::Other,
        }
    }

    #[test]
    fn format_bytes_picks_the_largest_unit() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(5 * 1024), "5 KB");
        assert_eq!(format_bytes(5 * 1024 * 1024), "5.0 MB");
        assert_eq!(format_bytes(3 * 1024 * 1024 * 1024), "3.00 GB");
    }

    #[test]
    fn bucket_percentages_are_shares_of_the_scanned_total() {
        let tree = StorageTree {
            bucket_totals: vec![
                (StorageBucket::Applications, 300),
                (StorageBucket::Windows, 100),
                (StorageBucket::Users, 0),
            ],
            ..Default::default()
        };

        let shares = bucket_percentages(&tree);
        assert_eq!(shares.len(), StorageBucket::ALL.len());
        assert_eq!(shares[0].0, StorageBucket::Applications);
        assert_eq!(shares[0].1, 300);
        assert!((shares[0].2 - 75.0).abs() < 0.01);
        assert!((shares[1].2 - 25.0).abs() < 0.01);
        // An unscanned tree reports 0%, never NaN.
        assert_eq!(shares[2].2, 0.0);
        assert_eq!(bucket_percentages(&StorageTree::default())[0].2, 0.0);
    }

    #[test]
    fn top_entries_ranks_and_truncates() {
        let tree = StorageTree {
            entries: vec![entry("small", 10), entry("big", 900), entry("mid", 100)],
            large_files: Vec::<LargeFile>::new(),
            ..Default::default()
        };

        let top = top_entries(&tree, 2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].name, "big");
        assert_eq!(top[1].name, "mid");

        assert!(top_entries(&tree, 0).is_empty());
        assert_eq!(top_entries(&tree, 99).len(), 3);
    }
}
