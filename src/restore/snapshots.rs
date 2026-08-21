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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    pub timestamp: String,
    pub description: String,
    pub registry_entries: Vec<RegistryBackupEntry>,
    pub service_entries: Vec<ServiceBackupEntry>,
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

pub fn create_snapshot(description: &str) -> Snapshot {
    let now = Local::now();
    let id = format!("{:x}", now.timestamp_millis());
    let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();

    let snapshot = Snapshot {
        id: id.clone(),
        timestamp,
        description: description.to_string(),
        registry_entries: Vec::new(),
        service_entries: Vec::new(),
    };

    let file_path = snapshots_dir().join(format!("{}.json", id));
    if let Ok(content) = serde_json::to_string_pretty(&snapshot) {
        let _ = fs::write(file_path, content);
    }

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
