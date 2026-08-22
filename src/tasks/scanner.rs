use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::tasks::rules::{find_rule, matches_third_party_heuristic, ScheduledTaskRule, TaskCategory};

/// Root directory holding every registered task's XML definition.
pub fn tasks_root() -> Option<PathBuf> {
    let windir = std::env::var("WinDir").ok()?;
    Some(PathBuf::from(windir).join("System32").join("Tasks"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTaskItem {
    /// Full task path for schtasks.exe, e.g. "\Microsoft\Windows\...\TaskName"
    pub task_path: String,
    pub name: String,
    pub category_label: String,
    pub category_is_safe: bool,
    pub description: String,
    pub is_enabled: bool,
    /// True when we successfully parsed the on-disk XML definition.
    pub xml_readable: bool,
}

/// Minimal extraction of the `<Enabled>` element from a task definition.
fn parse_enabled_flag(xml: &str) -> Option<bool> {
    let idx = xml.to_lowercase().find("<enabled>")?;
    let rest = &xml[idx + "<enabled>".len()..];
    let end = rest.find('<')?;
    match rest[..end].trim().to_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn classify(task_rel_path_lower: &str) -> (String, bool, String, TaskCategory) {
    if let Some(rule) = find_rule(task_rel_path_lower) {
        return (
            rule.category.as_str().to_string(),
            rule.category.is_safe_to_disable(),
            rule.description.to_string(),
            rule.category,
        );
    }
    (
        "Third-Party Updater".to_string(),
        true,
        "Detected third-party background updater or helper task.".to_string(),
        TaskCategory::ThirdPartyUpdate,
    )
}

fn walk(dir: &Path, rel: String, depth: u8, out: &mut Vec<ScheduledTaskItem>, budget: &mut usize) {
    if *budget == 0 || depth > 8 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else { return };

    for entry in entries.flatten() {
        if *budget == 0 {
            return;
        }
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let child_rel = format!("{}\\{}", rel, name);

        if path.is_dir() {
            walk(&path, child_rel, depth + 1, out, budget);
        } else {
            *budget -= 1;

            // Only inspect task-definition files that match our rule set or heuristics.
            let rel_lower = child_rel.to_lowercase();
            let curated = find_rule(&rel_lower);
            let third_party = !curated.is_some() && matches_third_party_heuristic(&rel_lower);
            if curated.is_none() && !third_party {
                continue;
            }

            let (xml_readable, is_enabled) = match fs::read_to_string(&path) {
                Ok(xml) => (true, parse_enabled_flag(&xml).unwrap_or(true)),
                Err(_) => {
                    // ACL-restricted definitions still exist — report as enabled
                    // since schtasks can toggle them regardless of readability.
                    (false, true)
                }
            };

            let (category_label, category_is_safe, description, _) = classify(&rel_lower);

            out.push(ScheduledTaskItem {
                task_path: child_rel,
                name: name.clone(),
                category_label,
                category_is_safe,
                description,
                is_enabled,
                xml_readable,
            });
        }
    }
}

/// Scan the scheduled-task store for known telemetry/CEIP/third-party tasks.
/// Reading the native XML definitions keeps this scan locale-proof (no
/// localized `schtasks /Query` text parsing).
pub fn scan_scheduled_tasks() -> Vec<ScheduledTaskItem> {
    let mut results = Vec::new();
    let Some(root) = tasks_root() else { return results };
    let mut budget: usize = 4096;
    walk(&root, String::new(), 0, &mut results, &mut budget);
    results.sort_by(|a, b| a.category_label.cmp(&b.category_label).then(a.name.cmp(&b.name)));
    results
}

/// Pure test helper re-exported for unit tests.
pub fn enabled_flag_from_xml_for_test(xml: &str) -> Option<bool> {
    parse_enabled_flag(xml)
}

#[allow(dead_code)]
fn _rule_type_assert(r: &ScheduledTaskRule) -> &str {
    r.task_path
}
