//! Recursive storage measurement.
//!
//! The analyzer answers "where did the space go" by walking a real filesystem,
//! which is the one thing in Wino that touches arbitrary user data. Three
//! constraints shape every function here:
//!
//! * **Nothing is ever written.** The walk opens directories, reads metadata,
//!   and returns numbers. Every reclaim path stays in the Storage Cleaner.
//! * **Reparse points are never followed.** `C:\Users\All Users`,
//!   `C:\Documents and Settings`, and every junction a vendor drops in a
//!   profile would otherwise make the walk visit the same subtree repeatedly —
//!   inflating totals and, on a cycle, never terminating. Both the `is_symlink`
//!   flag and `FILE_ATTRIBUTE_REPARSE_POINT` are checked, because a junction is
//!   reported as a directory with the attribute set, not as a symlink.
//! * **The walk always terminates.** An entry budget (`storage.max_entries`), a
//!   depth ceiling (`storage.max_depth`), and a cancellation token bound the
//!   work. Hitting a bound sets a flag on the result so the UI can present a
//!   partial tree as partial instead of as fact.
//!
//! IO errors never escape: a directory that denies access is counted in
//! `skipped_inaccessible` and the walk continues.

use std::fs;
use std::io;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::cleaner::scanner::load_cleanup_rules;
use crate::core::cancel::CancelToken;
use crate::core::config::{AppConfig, StorageConfig};
use crate::core::logger::{log_info, log_warn};
use crate::monitoring::disk::get_primary_disk_stats;
use crate::storage::models::{
    classify_path, is_large_file, is_old_file, CacheLocation, LargeFile, StorageBucket,
    StorageEntry, StorageTree,
};

/// `FILE_ATTRIBUTE_REPARSE_POINT` — set on symlinks, junctions, and mount points.
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;

/// Reported `large_files` / `old_files` entries per scan. A machine can hold
/// thousands of files over the threshold; the UI lists a sample, and the cap
/// keeps the scan's allocation flat.
const MAX_REPORTED_FILES: usize = 200;

/// Drive the primary-disk statistics describe. `get_primary_disk_stats` reports
/// `C:` only, so totals are filled in for that drive and left zero elsewhere.
const PRIMARY_DRIVE: &str = "c:";

/// Bounds applied to one walk, resolved once from configuration.
struct WalkLimits {
    max_entries: usize,
    max_depth: u8,
    large_file_mb: u64,
    old_file_days: u64,
}

impl WalkLimits {
    fn from_storage(storage: &StorageConfig) -> Self {
        Self {
            max_entries: storage.max_entries,
            max_depth: storage.max_depth,
            large_file_mb: storage.large_file_mb,
            old_file_days: storage.old_file_days,
        }
    }
}

/// Result of measuring one subtree.
#[derive(Debug, Clone, Copy, Default)]
struct Measure {
    bytes: u64,
    files: usize,
    /// True when the subtree was not fully measured (limit, cancellation,
    /// access denial, or the depth ceiling).
    partial: bool,
}

impl Measure {
    /// A subtree that was not measured at all, for a guard that stops the walk.
    fn skipped() -> Self {
        Self {
            partial: true,
            ..Self::default()
        }
    }
}

/// Mutable state carried through one walk.
///
/// `collect_reports` separates the main scan — which builds bucket totals and
/// the large/old file lists — from the small re-measurements of the cleaner's
/// cache directories, which need only a byte total and a file count.
struct WalkState<'a> {
    cancel: &'a CancelToken,
    limits: &'a WalkLimits,
    now: SystemTime,
    collect_reports: bool,
    entries_used: usize,
    files_scanned: usize,
    dirs_scanned: usize,
    skipped_inaccessible: usize,
    skipped_reparse: usize,
    limit_reached: bool,
    cancelled: bool,
    bucket_totals: [u64; StorageBucket::ALL.len()],
    large_files: Vec<LargeFile>,
    old_files: Vec<LargeFile>,
}

impl<'a> WalkState<'a> {
    fn new(cancel: &'a CancelToken, limits: &'a WalkLimits, collect_reports: bool) -> Self {
        Self {
            cancel,
            limits,
            now: SystemTime::now(),
            collect_reports,
            entries_used: 0,
            files_scanned: 0,
            dirs_scanned: 0,
            skipped_inaccessible: 0,
            skipped_reparse: 0,
            limit_reached: false,
            cancelled: false,
            bucket_totals: [0; StorageBucket::ALL.len()],
            large_files: Vec::new(),
            old_files: Vec::new(),
        }
    }

    /// True once the entry budget is spent, setting `limit_reached` as a side
    /// effect so the caller can stop without losing the reason.
    fn budget_exhausted(&mut self) -> bool {
        if self.entries_used >= self.limits.max_entries {
            self.limit_reached = true;
            return true;
        }
        false
    }

    fn note_file(&mut self, path: &Path, size: u64, modified: io::Result<SystemTime>) {
        self.files_scanned += 1;
        if !self.collect_reports {
            return;
        }

        let text = path.to_string_lossy();
        let bucket = classify_path(text.as_ref());
        let index = bucket_index(bucket);
        self.bucket_totals[index] = self.bucket_totals[index].saturating_add(size);

        let age_days = file_age_days(self.now, modified);
        let record = || LargeFile {
            path: text.to_string(),
            size_bytes: size,
            age_days,
        };

        if is_large_file(size, self.limits.large_file_mb) {
            self.large_files.push(record());
        }
        if is_old_file(age_days, self.limits.old_file_days) {
            self.old_files.push(record());
        }
    }

    fn bucket_total(&self, bucket: StorageBucket) -> u64 {
        self.bucket_totals[bucket_index(bucket)]
    }
}

fn bucket_index(bucket: StorageBucket) -> usize {
    StorageBucket::ALL
        .iter()
        .position(|candidate| *candidate == bucket)
        .unwrap_or(StorageBucket::ALL.len() - 1)
}

/// Days since `modified`, `None` when the timestamp is unreadable or in the
/// future. An unknown age is never treated as old.
fn file_age_days(now: SystemTime, modified: io::Result<SystemTime>) -> Option<u64> {
    let modified = modified.ok()?;
    let elapsed = now.duration_since(modified).ok()?;
    Some(elapsed.as_secs() / 86_400)
}

/// A reparse point must not be descended into: a junction is a directory with
/// `FILE_ATTRIBUTE_REPARSE_POINT`, a symlink is also flagged `is_symlink`.
fn is_reparse_point(meta: &fs::Metadata) -> bool {
    meta.file_type().is_symlink() || (meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT) != 0
}

/// Drive root of `path` (`C:\Windows` -> `C:\`), or `path` itself when it has no
/// drive prefix. The UI breadcrumb is relative to this.
fn drive_root(path: &str) -> String {
    let bytes = path.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' {
        format!("{}\\", &path[..2])
    } else {
        path.to_string()
    }
}

/// Resolve `%LOCALAPPDATA%`, `%WINDIR%`, and `%TEMP%` in a cleanup rule's
/// `path_template`.
///
/// Mirrors `cleaner::scanner`'s private resolver: that function is not exported,
/// and the analyzer must resolve the same templates to the same directories so
/// a reported cache size matches what the Cleaner would clean.
fn resolve_path_template(template: &str) -> PathBuf {
    let mut resolved = template.to_string();

    if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
        resolved = resolved.replace("%LOCALAPPDATA%", &localappdata);
    }
    if let Ok(windir) = std::env::var("WINDIR") {
        resolved = resolved.replace("%WINDIR%", &windir);
    }
    if let Ok(temp) = std::env::var("TEMP") {
        resolved = resolved.replace("%TEMP%", &temp);
    }

    PathBuf::from(resolved)
}

/// One child that survived the walk guards, already measured.
struct Child {
    name: String,
    path: String,
    is_dir: bool,
    measure: Measure,
}

/// Children of a directory, measured.
struct Children {
    items: Vec<Child>,
    /// True when a guard stopped the listing early or a child could not be read,
    /// so the caller's subtree is not fully measured.
    truncated: bool,
}

/// Read and measure `dir`'s children, applying every walk guard in one place so
/// a correction to a guard applies to the root listing and every nested subtree
/// at once. `child_depth` is the depth the children sit at.
///
/// `None` means the directory itself could not be opened.
fn read_children(state: &mut WalkState, dir: &Path, child_depth: u8) -> Option<Children> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => {
            state.skipped_inaccessible += 1;
            return None;
        }
    };
    state.dirs_scanned += 1;

    let mut children = Children {
        items: Vec::new(),
        truncated: false,
    };

    for child in entries {
        if state.cancel.is_cancelled() {
            state.cancelled = true;
            children.truncated = true;
            return Some(children);
        }
        if state.budget_exhausted() {
            children.truncated = true;
            return Some(children);
        }
        state.entries_used += 1;

        let Ok(child) = child else {
            state.skipped_inaccessible += 1;
            children.truncated = true;
            continue;
        };
        let path = child.path();

        let Ok(meta) = fs::symlink_metadata(&path) else {
            state.skipped_inaccessible += 1;
            children.truncated = true;
            continue;
        };
        // Skipping a reparse point is a deliberate choice, not a failed
        // measurement, so it does not mark the parent partial.
        if is_reparse_point(&meta) {
            state.skipped_reparse += 1;
            continue;
        }

        let is_dir = meta.is_dir();
        let measure = if is_dir {
            measure_subtree(state, &path, child_depth)
        } else {
            let size = meta.len();
            state.note_file(&path, size, meta.modified());
            Measure {
                bytes: size,
                files: 1,
                partial: false,
            }
        };

        children.items.push(Child {
            name: child.file_name().to_string_lossy().to_string(),
            path: path.to_string_lossy().to_string(),
            is_dir,
            measure,
        });
    }

    Some(children)
}

/// Recursive byte and file count for `dir`, honouring every walk bound.
///
/// `depth` is the depth of `dir` itself, with the scan root at 0. Hitting the
/// ceiling marks the result partial: the subtree is deliberately not measured,
/// and reporting a truncated size as complete would be a silent undercount.
fn measure_subtree(state: &mut WalkState, dir: &Path, depth: u8) -> Measure {
    if state.cancel.is_cancelled() {
        state.cancelled = true;
        return Measure::skipped();
    }
    if depth > state.limits.max_depth {
        return Measure::skipped();
    }

    let Some(children) = read_children(state, dir, depth.saturating_add(1)) else {
        return Measure::skipped();
    };

    let mut measure = Measure {
        partial: children.truncated,
        ..Measure::default()
    };
    for child in children.items {
        measure.bytes = measure.bytes.saturating_add(child.measure.bytes);
        measure.files += child.measure.files;
        measure.partial |= child.measure.partial;
    }

    measure
}

/// Direct children of `root`, each with its recursive size, in descending size
/// order. This is what the tree view renders for the current directory.
///
/// A truncated listing needs no flag here: the entries that were reached carry
/// their own `partial`, and a budget or cancellation stop is already recorded
/// on the tree itself.
fn walk_root(state: &mut WalkState, root: &Path) -> Vec<StorageEntry> {
    let Some(children) = read_children(state, root, 1) else {
        return Vec::new();
    };

    let mut entries: Vec<StorageEntry> = children
        .items
        .into_iter()
        .map(|child| {
            let bucket = classify_path(&child.path);
            StorageEntry {
                name: child.name,
                bucket,
                path: child.path,
                is_dir: child.is_dir,
                size_bytes: child.measure.bytes,
                file_count: child.measure.files,
                partial: child.measure.partial,
            }
        })
        .collect();

    entries.sort_by_key(|a| std::cmp::Reverse(a.size_bytes));
    entries
}

/// Largest first, capped. The walk collects every candidate, so the cap is
/// applied after ranking rather than to the first N found.
fn top_by_size(mut files: Vec<LargeFile>, limit: usize) -> Vec<LargeFile> {
    files.sort_by_key(|a| std::cmp::Reverse(a.size_bytes));
    files.truncate(limit);
    files
}

/// Measure every existing directory a cleanup rule targets.
///
/// Each location gets its own walk state, so its entries neither count against
/// the main scan's budget nor double-count into its bucket totals — the tree
/// reports the cache directories separately.
fn scan_cache_locations(cancel: &CancelToken, limits: &WalkLimits) -> Vec<CacheLocation> {
    let mut locations = Vec::new();

    for rule in load_cleanup_rules() {
        if cancel.is_cancelled() {
            break;
        }

        let resolved = resolve_path_template(&rule.path_template);
        let Ok(meta) = fs::symlink_metadata(&resolved) else {
            continue;
        };
        if !meta.is_dir() || is_reparse_point(&meta) {
            continue;
        }

        let mut state = WalkState::new(cancel, limits, false);
        let measure = measure_subtree(&mut state, &resolved, 1);

        locations.push(CacheLocation {
            rule_id: rule.id,
            label: rule.name,
            path: resolved.to_string_lossy().to_string(),
            size_bytes: measure.bytes,
            file_count: measure.files,
        });
    }

    locations
}

/// Scan the primary drive. Convenience wrapper for callers with nothing to
/// cancel.
pub fn scan_system_drive() -> StorageTree {
    scan_system_drive_cancellable(&CancelToken::new())
}

/// Scan the primary drive, stopping when `cancel` is signalled.
pub fn scan_system_drive_cancellable(cancel: &CancelToken) -> StorageTree {
    scan_directory("C:\\", cancel)
}

/// Scan an arbitrary directory: `entries` describes that directory's children,
/// and the drive totals are filled in only when it sits on the primary drive.
pub fn scan_directory(path: &str, cancel: &CancelToken) -> StorageTree {
    let config = AppConfig::load();
    let limits = WalkLimits::from_storage(&config.storage);

    let root_path = PathBuf::from(path);
    let mut state = WalkState::new(cancel, &limits, true);
    let entries = walk_root(&mut state, &root_path);

    let mut tree = StorageTree {
        root: drive_root(path),
        current_path: path.to_string(),
        entries,
        bucket_totals: StorageBucket::ALL
            .iter()
            .map(|bucket| (*bucket, state.bucket_total(*bucket)))
            .collect(),
        large_files: top_by_size(std::mem::take(&mut state.large_files), MAX_REPORTED_FILES),
        old_files: top_by_size(std::mem::take(&mut state.old_files), MAX_REPORTED_FILES),
        files_scanned: state.files_scanned,
        dirs_scanned: state.dirs_scanned,
        skipped_inaccessible: state.skipped_inaccessible,
        skipped_reparse: state.skipped_reparse,
        limit_reached: state.limit_reached,
        cancelled: state.cancelled,
        ..StorageTree::default()
    };

    // The cache list is a second phase, so a cancellation during it leaves the
    // tree's main data complete but the cache section truncated. Flagging that
    // is what stops the UI from presenting the short list as the whole picture.
    tree.cache_locations = scan_cache_locations(cancel, &limits);
    if cancel.is_cancelled() {
        tree.cancelled = true;
    }

    if path.to_lowercase().starts_with(PRIMARY_DRIVE) {
        let disk = get_primary_disk_stats();
        tree.total_bytes = disk.total_bytes;
        tree.free_bytes = disk.free_bytes;
        tree.used_bytes = disk.used_bytes;
    }

    if tree.limit_reached || tree.cancelled {
        log_warn(
            "storage",
            &format!(
                "Storage scan of {} stopped early (limit reached: {}, cancelled: {}): {} files, {} directories",
                path, tree.limit_reached, tree.cancelled, tree.files_scanned, tree.dirs_scanned
            ),
        );
    } else {
        log_info(
            "storage",
            &format!(
                "Storage scan of {} complete: {} files, {} directories, {} inaccessible, {} reparse points skipped",
                path,
                tree.files_scanned,
                tree.dirs_scanned,
                tree.skipped_inaccessible,
                tree.skipped_reparse
            ),
        );
    }

    tree
}

/// The `limit` largest files found by the scan, largest first.
pub fn largest_files_in(tree: &StorageTree, limit: usize) -> Vec<LargeFile> {
    top_by_size(tree.large_files.clone(), limit)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> WalkLimits {
        WalkLimits {
            max_entries: 1000,
            max_depth: 8,
            large_file_mb: 100,
            old_file_days: 365,
        }
    }

    #[test]
    fn top_by_size_ranks_then_caps() {
        let file = |name: &str, size: u64| LargeFile {
            path: name.to_string(),
            size_bytes: size,
            age_days: None,
        };

        let ranked = top_by_size(vec![file("a", 10), file("b", 900), file("c", 100)], 2);
        assert_eq!(ranked.len(), 2);
        assert_eq!(ranked[0].path, "b");
        assert_eq!(ranked[1].path, "c");
        assert!(top_by_size(Vec::new(), 5).is_empty());
    }

    #[test]
    fn drive_root_extracts_the_volume() {
        assert_eq!(drive_root(r"C:\Windows"), r"C:\");
        assert_eq!(drive_root("d:/data"), "d:\\");
        assert_eq!(drive_root(r"C:\"), r"C:\");
        assert_eq!(drive_root("relative\\dir"), "relative\\dir");
    }

    #[test]
    fn missing_directory_yields_a_partial_tree_instead_of_an_error() {
        let cancel = CancelToken::new();
        let limits = limits();
        let mut state = WalkState::new(&cancel, &limits, true);

        let entries = walk_root(&mut state, Path::new(r"C:\__wino_missing_directory__"));
        assert!(entries.is_empty());
        assert_eq!(state.skipped_inaccessible, 1);
        assert_eq!(state.files_scanned, 0);
    }

    #[test]
    fn entry_budget_stops_the_walk_and_flags_it() {
        let cancel = CancelToken::new();
        let no_budget = WalkLimits {
            max_entries: 0,
            ..limits()
        };
        let mut state = WalkState::new(&cancel, &no_budget, true);

        assert!(state.budget_exhausted());
        assert!(state.limit_reached);

        // A cancelled token is observed before any directory is opened.
        let cancel = CancelToken::new();
        cancel.cancel();
        let walk_limits = limits();
        let mut state = WalkState::new(&cancel, &walk_limits, true);
        let measure = measure_subtree(&mut state, Path::new(r"C:\Windows"), 1);
        assert!(measure.partial);
        assert!(state.cancelled);
        assert_eq!(state.dirs_scanned, 0);
    }

    #[test]
    fn unknown_age_is_never_old_and_bucket_totals_stay_in_display_order() {
        let cancel = CancelToken::new();
        let limits = limits();
        let mut state = WalkState::new(&cancel, &limits, true);

        state.note_file(
            Path::new(r"C:\Windows\System32\driver.sys"),
            4096,
            Err(io::Error::other("no timestamp")),
        );
        state.note_file(
            Path::new(r"C:\Users\Ada\notes.txt"),
            10,
            Err(io::Error::other("no timestamp")),
        );

        assert!(state.old_files.is_empty());
        assert!(state.large_files.is_empty());
        assert_eq!(state.files_scanned, 2);
        assert_eq!(state.bucket_total(StorageBucket::Windows), 4096);
        assert_eq!(state.bucket_total(StorageBucket::Users), 10);
        assert_eq!(state.bucket_total(StorageBucket::Temp), 0);

        let ordered: Vec<StorageBucket> = StorageBucket::ALL
            .iter()
            .map(|bucket| (*bucket, state.bucket_total(*bucket)))
            .map(|(bucket, _)| bucket)
            .collect();
        assert_eq!(ordered, StorageBucket::ALL.to_vec());
    }
}
