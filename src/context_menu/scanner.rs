use serde::{Deserialize, Serialize};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CLASSES_ROOT, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE,
};

use crate::core::regutil;

/// Where a context-menu handler entry physically lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HandlerStore {
    /// HKLM\Software\Classes (machine-wide)
    Machine,
    /// HKCU\Software\Classes (per-user)
    User,
}

impl HandlerStore {
    pub fn hive(&self) -> HKEY {
        match self {
            HandlerStore::Machine => HKEY_LOCAL_MACHINE,
            HandlerStore::User => HKEY_CURRENT_USER,
        }
    }

    pub fn base_path(&self) -> &'static str {
        match self {
            HandlerStore::Machine => "Software\\Classes",
            HandlerStore::User => "Software\\Classes",
        }
    }

    /// Hive string used for snapshot bookkeeping.
    pub fn hive_label(&self) -> &'static str {
        match self {
            HandlerStore::Machine => "HKLM",
            HandlerStore::User => "HKCU",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMenuEntry {
    /// Logical shell root this handler attaches to: "*", "Directory", "Directory\\Background", "Folder", "AllFilesystemObjects"
    pub shell_root: String,
    /// Full registry sub-path relative to the store's Software\Classes
    /// e.g. "*\shellex\ContextMenuHandlers\Sync Center Conflict Handlers"
    pub key_path: String,
    pub handler_name: String,
    pub clsid: String,
    pub friendly_name: String,
    pub dll_path: String,
    pub is_enabled: bool,
    pub store: HandlerStore,
}

/// The Explorer shell roots scanned for slow/dead context menu handlers.
pub const SHELL_ROOTS: &[&str] = &[
    "*",
    "AllFilesystemObjects",
    "Directory",
    "Directory\\Background",
    "Folder",
];

const HANDLERS_SUFFIX: &str = "\\shellex\\ContextMenuHandlers";

/// Resolve a CLSID to (friendly name, inproc server dll).
fn resolve_clsid(clsid: &str) -> (String, String) {
    let path = format!("CLSID\\{}", clsid);
    let name =
        regutil::read_string(HKEY_CLASSES_ROOT, &path, "").unwrap_or_else(|| clsid.to_string());
    let dll = regutil::read_string(
        HKEY_CLASSES_ROOT,
        &format!("CLSID\\{}\\InprocServer32", clsid),
        "",
    )
    .unwrap_or_default();
    (name, dll)
}

/// Determine which physical hive holds the handler key. HKCR merges
/// HKLM\Software\Classes and HKCU\Software\Classes — we must write where the
/// entry actually lives.
fn detect_store(key_path: &str) -> Option<HandlerStore> {
    if regutil::read_string(
        HKEY_LOCAL_MACHINE,
        &format!("Software\\Classes\\{}", key_path),
        "",
    )
    .is_some()
    {
        return Some(HandlerStore::Machine);
    }
    if regutil::read_string(
        HKEY_CURRENT_USER,
        &format!("Software\\Classes\\{}", key_path),
        "",
    )
    .is_some()
    {
        return Some(HandlerStore::User);
    }
    // Fall back to machine even if only the key itself (no default value) exists there.
    if !regutil::enum_subkeys(
        HKEY_LOCAL_MACHINE,
        &format!("Software\\Classes\\{}", parent_of(key_path)),
    )
    .is_empty()
        || regutil::enum_subkeys(
            HKEY_CURRENT_USER,
            &format!("Software\\Classes\\{}", parent_of(key_path)),
        )
        .contains(&leaf_of(key_path).to_string())
    {
        return Some(HandlerStore::Machine);
    }
    None
}

fn parent_of(path: &str) -> &str {
    match path.rfind('\\') {
        Some(i) => &path[..i],
        None => "",
    }
}

fn leaf_of(path: &str) -> &str {
    match path.rfind('\\') {
        Some(i) => &path[i + 1..],
        None => path,
    }
}

/// Scan every known Explorer context menu handler registration point.
pub fn scan_context_menu_handlers() -> Vec<ContextMenuEntry> {
    let mut entries = Vec::new();

    for root in SHELL_ROOTS {
        let handlers_root = format!("{}{}", root, HANDLERS_SUFFIX);

        // Enumerate from merged HKCR view so we see both stores.
        for handler_key in regutil::enum_subkeys(HKEY_CLASSES_ROOT, &handlers_root) {
            let key_path = format!("{}\\{}", handlers_root, handler_key);
            let raw_default = regutil::read_string(HKEY_CLASSES_ROOT, &key_path, "");

            let Some(raw) = raw_default else { continue };
            if raw.trim().is_empty() {
                continue;
            }

            let is_enabled = !raw.trim_start().starts_with('-');
            let clsid = raw.trim().trim_start_matches('-').to_string();
            if !clsid.starts_with('{') {
                continue;
            }

            let (friendly_name, dll_path) = resolve_clsid(&clsid);

            // Prefer HKLM entries; skip duplicates surfaced by the merged view.
            if entries
                .iter()
                .any(|e: &ContextMenuEntry| e.key_path == key_path)
            {
                continue;
            }

            let store = detect_store(&key_path).unwrap_or(HandlerStore::Machine);

            entries.push(ContextMenuEntry {
                shell_root: root.to_string(),
                key_path,
                handler_name: handler_key,
                clsid,
                friendly_name,
                dll_path,
                is_enabled,
                store,
            });
        }
    }

    entries.sort_by(|a, b| {
        a.shell_root
            .cmp(&b.shell_root)
            .then(a.friendly_name.cmp(&b.friendly_name))
    });
    entries
}
