use crate::context_menu::scanner::{ContextMenuEntry, HandlerStore};
use crate::core::logger::{log_error, log_info};
use crate::core::regutil;
use crate::restore::snapshots::{create_snapshot_with_string_entry, StringBackupEntry};

/// Toggle a context-menu handler on/off.
///
/// Disabling rewrites the key's default CLSID value with a leading `-`
/// (Explorer ignores prefixed handlers). You can reverse it. It keeps
/// the registration data. Enabling strips the prefix.
pub fn toggle_handler(entry: &ContextMenuEntry, enable: bool, dry_run: bool) -> Result<(), String> {
    let action = if enable { "enable" } else { "disable" };
    let target = format!("{}\\{}", entry.store.hive_label(), entry.key_path);

    if entry.is_enabled == enable {
        return Err(format!(
            "Handler '{}' is already {}",
            entry.friendly_name,
            if enable { "enabled" } else { "disabled" }
        ));
    }

    let new_value = if enable {
        format!("{{{}}}", entry.clsid.trim_matches(|c| c == '{' || c == '}'))
    } else {
        format!(
            "-{{{}}}",
            entry.clsid.trim_matches(|c| c == '{' || c == '}')
        )
    };

    if dry_run {
        log_info(
            "context_menu",
            &format!(
                "[DRY-RUN] Would {} {} -> default value '{}'",
                action, target, new_value
            ),
        );
        return Ok(());
    }

    // Backup the current default value before mutating.
    let prev = regutil::read_string(entry.store.hive(), &full_path(entry), "");
    create_snapshot_with_string_entry(
        &format!("Context menu {}: {}", action, entry.friendly_name),
        &StringBackupEntry {
            hive: entry.store.hive_label().to_string(),
            path: full_path(entry),
            value_name: String::new(),
            previous_value: prev,
        },
    );

    match regutil::write_string(entry.store.hive(), &full_path(entry), "", &new_value) {
        Ok(()) => {
            log_info(
                "context_menu",
                &format!("{}d handler: {} ({})", action, entry.friendly_name, target),
            );
            Ok(())
        }
        Err(e) => {
            log_error(
                "context_menu",
                &format!("Failed to {} {}: {}", action, target, e),
            );
            Err(e)
        }
    }
}

fn full_path(entry: &ContextMenuEntry) -> String {
    format!("{}\\{}", entry.store.base_path(), entry.key_path)
}

/// Convenience helper for UI toggles: rebuild the "current" state check.
pub fn is_entry_enabled_now(entry: &ContextMenuEntry) -> bool {
    regutil::read_string(entry.store.hive(), &full_path(entry), "")
        .map(|v| !v.trim_start().starts_with('-'))
        .unwrap_or(entry.is_enabled)
}

#[allow(dead_code)]
fn _assert_store_copy(s: HandlerStore) -> HandlerStore {
    s
}
