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

/// Backup of a scheduled task's state before Wino changed it.
///
/// The full XML definition is stored, not just the enabled flag: restoring a
/// task reliably needs its original triggers, actions, and settings, and a
/// boolean alone cannot rebuild them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskBackupEntry {
    pub task_path: String,
    pub previous_enabled: bool,
    #[serde(default)]
    pub xml_definition: Option<String>,
    #[serde(default)]
    pub xml_readable: bool,
}

/// Backup of a power setting's AC/DC values before Wino changed them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerBackupEntry {
    /// Power scheme GUID the values belonged to (uppercase, no braces).
    pub scheme_guid: String,
    /// Subgroup GUID (uppercase, no braces).
    pub subgroup_guid: String,
    /// Setting GUID (uppercase, no braces).
    pub setting_guid: String,
    pub setting_name: String,
    pub previous_ac_value: Option<u32>,
    pub previous_dc_value: Option<u32>,
}

/// Backup of a network adapter setting before Wino changed it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkBackupEntry {
    /// Interface GUID including braces, as stored under Tcpip\Parameters\Interfaces.
    pub interface_guid: String,
    pub value_name: String,
    pub previous_value: Option<String>,
    /// Human label for the UI, e.g. the adapter friendly name.
    #[serde(default)]
    pub adapter_label: String,
}

/// Snapshot of the profile that was active before Wino applied a profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileBackupEntry {
    pub profile_id: String,
    pub profile_name: String,
    /// Step ids that were applied, so a rollback can list what it undoes.
    #[serde(default)]
    pub applied_steps: Vec<String>,
}

/// Categories a snapshot can cover. Used by the UI to show what a snapshot
/// actually recorded and whether it is restorable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SnapshotCategory {
    Registry,
    Services,
    Tasks,
    ContextMenu,
    Power,
    Network,
    Profiles,
}

impl SnapshotCategory {
    pub fn i18n_key(&self) -> &'static str {
        match self {
            SnapshotCategory::Registry => "restore.cat_registry",
            SnapshotCategory::Services => "restore.cat_services",
            SnapshotCategory::Tasks => "restore.cat_tasks",
            SnapshotCategory::ContextMenu => "restore.cat_context_menu",
            SnapshotCategory::Power => "restore.cat_power",
            SnapshotCategory::Network => "restore.cat_network",
            SnapshotCategory::Profiles => "restore.cat_profiles",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            SnapshotCategory::Registry => "Registry",
            SnapshotCategory::Services => "Services",
            SnapshotCategory::Tasks => "Scheduled Tasks",
            SnapshotCategory::ContextMenu => "Context Menu",
            SnapshotCategory::Power => "Power Settings",
            SnapshotCategory::Network => "Network Settings",
            SnapshotCategory::Profiles => "Profiles",
        }
    }

    pub fn color_rgb(&self) -> (u8, u8, u8) {
        match self {
            SnapshotCategory::Registry => (60, 144, 255),
            SnapshotCategory::Services => (78, 222, 163),
            SnapshotCategory::Tasks => (255, 185, 95),
            SnapshotCategory::ContextMenu => (167, 139, 250),
            SnapshotCategory::Power => (236, 72, 153),
            SnapshotCategory::Network => (56, 189, 248),
            SnapshotCategory::Profiles => (250, 204, 21),
        }
    }
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
    #[serde(default)]
    pub power_entries: Vec<PowerBackupEntry>,
    #[serde(default)]
    pub network_entries: Vec<NetworkBackupEntry>,
    #[serde(default)]
    pub profile_entries: Vec<ProfileBackupEntry>,
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
            power_entries: Vec::new(),
            network_entries: Vec::new(),
            profile_entries: Vec::new(),
        }
    }

    /// Total number of recorded restore operations across every category.
    pub fn operation_count(&self) -> usize {
        self.registry_entries.len()
            + self.service_entries.len()
            + self.string_entries.len()
            + self.task_entries.len()
            + self.power_entries.len()
            + self.network_entries.len()
            + self.profile_entries.len()
    }

    /// Categories this snapshot actually recorded, in a stable order.
    pub fn categories(&self) -> Vec<SnapshotCategory> {
        let mut out = Vec::new();
        if !self.registry_entries.is_empty() {
            out.push(SnapshotCategory::Registry);
        }
        if !self.service_entries.is_empty() {
            out.push(SnapshotCategory::Services);
        }
        if !self.task_entries.is_empty() {
            out.push(SnapshotCategory::Tasks);
        }
        if !self.string_entries.is_empty() {
            out.push(SnapshotCategory::ContextMenu);
        }
        if !self.power_entries.is_empty() {
            out.push(SnapshotCategory::Power);
        }
        if !self.network_entries.is_empty() {
            out.push(SnapshotCategory::Network);
        }
        if !self.profile_entries.is_empty() {
            out.push(SnapshotCategory::Profiles);
        }
        out
    }

    /// Whether a rollback would do anything.
    pub fn is_restorable(&self) -> bool {
        self.operation_count() > 0
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

fn next_snapshot_id() -> (String, String) {
    let now = Local::now();
    let id = format!(
        "{:x}-{}",
        now.timestamp_millis(),
        now.timestamp_subsec_millis()
    );
    let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();
    (id, timestamp)
}

pub fn create_snapshot(description: &str) -> Snapshot {
    let (id, timestamp) = next_snapshot_id();
    let snapshot = Snapshot::new(id, timestamp, description);
    write_snapshot(&snapshot);
    snapshot
}

/// Create a snapshot pre-seeded with one REG_SZ backup entry.
pub fn create_snapshot_with_string_entry(description: &str, entry: &StringBackupEntry) -> Snapshot {
    let (id, timestamp) = next_snapshot_id();
    let mut snapshot = Snapshot::new(id, timestamp, description);
    snapshot.string_entries.push(entry.clone());
    write_snapshot(&snapshot);
    snapshot
}

/// Create a snapshot recording scheduled-task state changes.
pub fn create_snapshot_for_tasks(description: &str, entries: &[TaskBackupEntry]) -> Snapshot {
    let (id, timestamp) = next_snapshot_id();
    let mut snapshot = Snapshot::new(id, timestamp, description);
    snapshot.task_entries.extend_from_slice(entries);
    write_snapshot(&snapshot);
    snapshot
}

/// Create a snapshot recording a power setting change.
pub fn create_snapshot_for_power(description: &str, entries: &[PowerBackupEntry]) -> Snapshot {
    let (id, timestamp) = next_snapshot_id();
    let mut snapshot = Snapshot::new(id, timestamp, description);
    snapshot.power_entries.extend_from_slice(entries);
    write_snapshot(&snapshot);
    snapshot
}

/// Create a snapshot recording a network setting change.
pub fn create_snapshot_for_network(description: &str, entries: &[NetworkBackupEntry]) -> Snapshot {
    let (id, timestamp) = next_snapshot_id();
    let mut snapshot = Snapshot::new(id, timestamp, description);
    snapshot.network_entries.extend_from_slice(entries);
    write_snapshot(&snapshot);
    snapshot
}

/// Create a snapshot recording a profile application.
pub fn create_snapshot_for_profile(description: &str, entry: &ProfileBackupEntry) -> Snapshot {
    let (id, timestamp) = next_snapshot_id();
    let mut snapshot = Snapshot::new(id, timestamp, description);
    snapshot.profile_entries.push(entry.clone());
    write_snapshot(&snapshot);
    snapshot
}

/// Build a snapshot in memory without writing it (used by previews and tests).
pub fn build_snapshot(description: &str) -> Snapshot {
    let (id, timestamp) = next_snapshot_id();
    Snapshot::new(id, timestamp, description)
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

/// Look up one snapshot by id.
pub fn find_snapshot(snapshot_id: &str) -> Option<Snapshot> {
    list_snapshots().into_iter().find(|s| s.id == snapshot_id)
}

/// Write a snapshot to `destination` as JSON, for the "Export" action.
pub fn export_snapshot(snapshot_id: &str, destination: &str) -> Result<String, String> {
    let Some(snapshot) = find_snapshot(snapshot_id) else {
        return Err(format!("Snapshot ID '{}' not found.", snapshot_id));
    };

    let content = serde_json::to_string_pretty(&snapshot)
        .map_err(|e| format!("Failed to serialize snapshot: {}", e))?;

    let mut path = PathBuf::from(destination);
    if path.is_dir() {
        path = path.join(format!("wino-snapshot-{}.json", snapshot.id));
    }

    fs::write(&path, content)
        .map_err(|e| format!("Failed to write '{}': {}", path.display(), e))?;

    Ok(format!(
        "Snapshot '{}' exported to {}.",
        snapshot.id,
        path.display()
    ))
}

/// Delete a snapshot file. The audit log keeps the record that it existed.
pub fn delete_snapshot(snapshot_id: &str) -> Result<String, String> {
    let path = snapshots_dir().join(format!("{}.json", snapshot_id));
    if !path.exists() {
        return Err(format!("Snapshot ID '{}' not found.", snapshot_id));
    }

    fs::remove_file(&path).map_err(|e| format!("Failed to delete '{}': {}", path.display(), e))?;
    crate::core::logger::log_info("restore", &format!("Deleted snapshot {}", snapshot_id));
    Ok(format!("Snapshot '{}' deleted.", snapshot_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_count_and_categories_reflect_recorded_entries() {
        let mut snap = build_snapshot("test");
        assert_eq!(snap.operation_count(), 0);
        assert!(!snap.is_restorable());
        assert!(snap.categories().is_empty());

        snap.registry_entries.push(RegistryBackupEntry {
            hive: "HKCU".to_string(),
            path: "Software\\Test".to_string(),
            value_name: "Flag".to_string(),
            previous_value: Some(1),
        });
        snap.power_entries.push(PowerBackupEntry {
            scheme_guid: "AAAA".to_string(),
            subgroup_guid: "BBBB".to_string(),
            setting_guid: "CCCC".to_string(),
            setting_name: "Processor max".to_string(),
            previous_ac_value: Some(100),
            previous_dc_value: Some(50),
        });

        assert_eq!(snap.operation_count(), 2);
        assert!(snap.is_restorable());
        assert_eq!(
            snap.categories(),
            vec![SnapshotCategory::Registry, SnapshotCategory::Power]
        );
    }

    #[test]
    fn task_entry_keeps_xml_definition_for_faithful_restore() {
        let entry = TaskBackupEntry {
            task_path: "\\Microsoft\\Windows\\Test".to_string(),
            previous_enabled: true,
            xml_definition: Some(
                "<Task><Settings><Enabled>true</Enabled></Settings></Task>".to_string(),
            ),
            xml_readable: true,
        };
        let json = serde_json::to_string(&entry).expect("serialize task entry");
        let back: TaskBackupEntry = serde_json::from_str(&json).expect("deserialize task entry");
        assert_eq!(back.task_path, entry.task_path);
        assert!(back.xml_readable);
        assert!(back
            .xml_definition
            .as_deref()
            .unwrap_or("")
            .contains("<Task>"));
    }

    #[test]
    fn legacy_task_entry_without_xml_still_deserializes() {
        let legacy = r#"{"task_path":"\\Test","previous_enabled":true}"#;
        let back: TaskBackupEntry = serde_json::from_str(legacy).expect("legacy task entry");
        assert!(back.previous_enabled);
        assert!(back.xml_definition.is_none());
        assert!(!back.xml_readable);
    }
}
