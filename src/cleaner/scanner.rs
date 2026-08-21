use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub path_template: String,
    pub glob_pattern: String,
    pub risk: String,
    pub safe_to_clean: bool,
    pub min_age_hours: u64,
    pub description_benefit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedCleanItem {
    pub rule: CleanupRule,
    pub resolved_path: PathBuf,
    pub total_bytes: u64,
    pub file_count: usize,
    pub target_files: Vec<PathBuf>,
}

pub fn load_cleanup_rules() -> Vec<CleanupRule> {
    const CLEANUP_JSON: &str = include_str!("../../data/cleanup_rules.json");
    serde_json::from_str(CLEANUP_JSON).unwrap_or_default()
}

pub fn scan_cleaner_targets() -> Vec<ScannedCleanItem> {
    let rules = load_cleanup_rules();
    let mut results = Vec::new();

    for rule in rules {
        let resolved = resolve_path_template(&rule.path_template);
        if !resolved.exists() || !resolved.is_dir() {
            continue;
        }

        let mut total_bytes = 0u64;
        let mut target_files = Vec::new();

        scan_directory_files(&resolved, rule.min_age_hours, &mut total_bytes, &mut target_files);

        let file_count = target_files.len();
        if file_count > 0 {
            results.push(ScannedCleanItem {
                rule,
                resolved_path: resolved,
                total_bytes,
                file_count,
                target_files,
            });
        }
    }

    results
}

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

fn scan_directory_files(dir: &Path, min_age_hours: u64, total_bytes: &mut u64, target_files: &mut Vec<PathBuf>) {
    let now = SystemTime::now();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Ok(metadata) = entry.metadata() {
                    let is_old_enough = if min_age_hours > 0 {
                        if let Ok(modified) = metadata.modified() {
                            if let Ok(elapsed) = now.duration_since(modified) {
                                elapsed.as_secs() >= min_age_hours * 3600
                            } else {
                                false
                            }
                        } else {
                            true
                        }
                    } else {
                        true
                    };

                    if is_old_enough {
                        *total_bytes += metadata.len();
                        target_files.push(path);
                    }
                }
            } else if path.is_dir() {
                // Scan subdirectories up to reasonable depth
                scan_directory_files(&path, min_age_hours, total_bytes, target_files);
            }
        }
    }
}
