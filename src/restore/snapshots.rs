use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryBackupEntry {
    pub hive: String,
    pub path: String,
    pub value_name: String,
    pub previous_value: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceBackupEntry {
    pub service_name: String,
    pub previous_startup: String,
}

/// Backup entry for REG_SZ values (empty `value_name` = key default value).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringBackupEntry {
    pub hive: String,
    pub path: String,
    pub value_name: String,
    pub previous_value: Option<String>,
}

/// Backup of a scheduled task XML definition (captured before disabling).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskBackupEntry {
    pub task_path: String,
    pub previous_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    pub timestamp: String,
    pub description: String,
    pub registry_entries: Vec<RegistryBackupEntry>,
    pub service_entries: Vec<ServiceBackupEntry>,
    #[serde(default)]
    pub string_entries: Vec<StringBackupEntry>,
    #[serde(default)]
    pub task_entries: Vec<TaskBackupEntry>,
}

impl Snapshot {
    fn new(id: String, timestamp: String, description: &str) -> Self {
        Self {
            id,
            timestamp,
            description: description.to_string(),
            registry_entries: Vec::new(),
            service_entries: Vec::new(),
            string_entries: Vec::new(),
            task_entries: Vec::new(),
        }
    }
}

pub fn snapshots_dir() -> PathBuf {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        let p = PathBuf::from(appdata).join("Wino").join("snapshots");
        let _ = fs::create_dir_all(&p);
        p
    } else {
        let p = PathBuf::from("snapshots");
        let _ = fs::create_dir_all(&p);
        p
    }
}

fn write_snapshot(snapshot: &Snapshot) {
    let file_path = snapshots_dir().join(format!("{}.json", snapshot.id));
    if let Ok(content) = serde_json::to_string_pretty(snapshot) {
        let _ = fs::write(file_path, content);
    }
}

pub fn create_snapshot(description: &str) -> Snapshot {
    let now = Local::now();
    let id = format!("{:x}", now.timestamp_millis());
    let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();

    let snapshot = Snapshot::new(id.clone(), timestamp, description);
    write_snapshot(&snapshot);
    snapshot
}

/// Create a snapshot pre-seeded with one REG_SZ backup entry.
pub fn create_snapshot_with_string_entry(description: &str, entry: &StringBackupEntry) -> Snapshot {
    let now = Local::now();
    let id = format!("{:x}-{}", now.timestamp_millis(), now.timestamp_subsec_millis());
    let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();

    let mut snapshot = Snapshot::new(id.clone(), timestamp, description);
    snapshot.string_entries.push(entry.clone());
    write_snapshot(&snapshot);
    snapshot
}

/// Create a snapshot recording scheduled-task state changes.
pub fn create_snapshot_for_tasks(description: &str, entries: &[TaskBackupEntry]) -> Snapshot {
    let now = Local::now();
    let id = format!("{:x}-{}", now.timestamp_millis(), now.timestamp_subsec_millis());
    let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();

    let mut snapshot = Snapshot::new(id.clone(), timestamp, description);
    snapshot.task_entries.extend_from_slice(entries);
    write_snapshot(&snapshot);
    snapshot
}

pub fn list_snapshots() -> Vec<Snapshot> {
    let mut results = Vec::new();
    let dir = snapshots_dir();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(snapshot) = serde_json::from_str::<Snapshot>(&content) {
                        results.push(snapshot);
                    }
                }
            }
        }
    }

    results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    results
}
