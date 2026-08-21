use crate::cleaner::scanner::ScannedCleanItem;
use crate::core::logger::log_info;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanerReport {
    pub freed_bytes: u64,
    pub files_deleted: usize,
    pub errors_count: usize,
    pub is_dry_run: bool,
    pub message: String,
}

pub fn execute_cleanup(items: &[ScannedCleanItem], dry_run: bool) -> CleanerReport {
    let mut total_freed = 0u64;
    let mut deleted_count = 0usize;
    let mut error_count = 0usize;

    for item in items {
        if dry_run {
            total_freed += item.total_bytes;
            deleted_count += item.file_count;
            continue;
        }

        for file in &item.target_files {
            if !crate::security::validation::is_safe_deletion_target(file) {
                continue;
            }

            if let Ok(meta) = fs::metadata(file) {
                let size = meta.len();
                if fs::remove_file(file).is_ok() {
                    total_freed += size;
                    deleted_count += 1;
                } else {
                    // File is likely locked by a running process
                    error_count += 1;
                }
            }
        }
    }

    let message = if dry_run {
        format!(
            "Dry run complete: Found {:.2} GB reclaimable across {} temporary files.",
            total_freed as f64 / (1024.0 * 1024.0 * 1024.0),
            deleted_count
        )
    } else {
        format!(
            "Cleanup complete: Reclaimed {:.2} MB across {} files ({} files in use were safely skipped).",
            total_freed as f64 / (1024.0 * 1024.0),
            deleted_count,
            error_count
        )
    };

    log_info("cleaner", &message);

    CleanerReport {
        freed_bytes: total_freed,
        files_deleted: deleted_count,
        errors_count: error_count,
        is_dry_run: dry_run,
        message,
    }
}
