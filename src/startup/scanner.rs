//! Startup discovery: every place Windows launches something at sign-in.
//!
//! The `enabled` flag is why this module looks the way it does. It used to
//! report every discovered entry as enabled, so the UI claimed a disabled
//! Spotify entry would start with Windows. Windows keeps that fact in the
//! `StartupApproved` keys as a 12-byte binary record per entry, read and applied
//! here — see [`parse_startup_approved`].
//!
//! Sources: `Run`/`RunOnce` for both hives (plus WOW6432Node), the two Startup
//! folders, `\Microsoft\Windows\` tasks with a logon or boot trigger, and the
//! `Winlogon` `Shell`/`Userinit` values. The last two are reported read-only:
//! they are security-sensitive and `startup::manager` refuses to toggle them.
//!
//! Discovery is bounded — the task walk stops after [`TASK_FILE_BUDGET`] files
//! and logs that it did — so a machine with thousands of tasks cannot hang the
//! scan.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{ERROR_MORE_DATA, ERROR_SUCCESS};
use windows::Win32::System::Registry::{
    RegCloseKey, RegEnumValueW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE,
    KEY_READ, REG_BINARY, REG_EXPAND_SZ, REG_SZ,
};

/// One entry Windows would launch at sign-in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupItem {
    pub name: String,
    pub command: String,
    pub exe_path: String,
    /// One of the `SOURCE_*` labels below. The label decides what a toggle does.
    pub source: String,
    /// Whether Windows would actually launch this entry, measured from the
    /// `StartupApproved` record (registry and folder entries) or the task's own
    /// `<Enabled>` element.
    pub enabled: bool,
    pub impact: String, // "Low", "Medium", "High"
    pub is_signed: bool,
    pub publisher: String,
}

/// Source labels. Public because `startup::manager` maps them back to the
/// registry location they came from; two copies would drift apart.
pub const SOURCE_HKCU_RUN: &str = "Registry (HKCU)";
pub const SOURCE_HKLM_RUN: &str = "Registry (HKLM)";
pub const SOURCE_HKLM_WOW64_RUN: &str = "Registry (HKLM WOW6432)";
pub const SOURCE_HKCU_RUN_ONCE: &str = "Registry (HKCU RunOnce)";
pub const SOURCE_HKLM_RUN_ONCE: &str = "Registry (HKLM RunOnce)";
pub const SOURCE_FOLDER_USER: &str = "Startup Folder (User)";
pub const SOURCE_FOLDER_COMMON: &str = "Startup Folder (Common)";
pub const SOURCE_SCHEDULED_TASK: &str = "Scheduled Task";
/// Read-only: Wino never toggles these.
pub const SOURCE_WINLOGON: &str = "Winlogon";

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const RUN_ONCE_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\RunOnce";
const RUN_WOW64_KEY: &str = "Software\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Run";
const APPROVAL_PREFIX: &str =
    "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\";
const WINLOGON_KEY: &str = "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Winlogon";

/// Upper bound on task definition files inspected per scan.
pub const TASK_FILE_BUDGET: usize = 2000;

/// Registry locations to scan, as (hive, subkey, source label, `StartupApproved`
/// subkey, is `RunOnce`).
///
/// `RunOnce` has its own approval key: reusing the `Run` map would read a
/// same-named `Run` entry's state as this entry's. `RunOnce` entries run once and
/// delete themselves, so they always report Low impact.
const REGISTRY_SOURCES: &[(HKEY, &str, &str, &str, bool)] = &[
    (HKEY_CURRENT_USER, RUN_KEY, SOURCE_HKCU_RUN, "Run", false),
    (HKEY_LOCAL_MACHINE, RUN_KEY, SOURCE_HKLM_RUN, "Run", false),
    (
        HKEY_LOCAL_MACHINE,
        RUN_WOW64_KEY,
        SOURCE_HKLM_WOW64_RUN,
        "Run32",
        false,
    ),
    (
        HKEY_CURRENT_USER,
        RUN_ONCE_KEY,
        SOURCE_HKCU_RUN_ONCE,
        "RunOnce",
        true,
    ),
    (
        HKEY_LOCAL_MACHINE,
        RUN_ONCE_KEY,
        SOURCE_HKLM_RUN_ONCE,
        "RunOnce",
        true,
    ),
];

/// Hive and subkey a toggleable registry source writes to, or `None` for a
/// source Wino must not touch (Winlogon, scheduled tasks, folders).
///
/// Derived from [`REGISTRY_SOURCES`] rather than restated, so a new registry
/// source cannot be discovered but un-toggleable, or toggled from the wrong key.
pub fn registry_source_target(source: &str) -> Option<(&'static str, &'static str)> {
    REGISTRY_SOURCES
        .iter()
        .find(|&&(_, _, label, _, _)| label == source)
        .map(|&(hive, subkey, _, _, _)| (hive_label(hive), subkey))
}

/// The label `SystemExecutor` accepts for a hive.
fn hive_label(hive: HKEY) -> &'static str {
    if hive == HKEY_CURRENT_USER {
        "HKCU"
    } else {
        "HKLM"
    }
}

/// Every startup entry discoverable on this machine.
pub fn scan_startup_items() -> Vec<StartupItem> {
    let mut items = Vec::new();

    for &(hive, subkey, label, approval_subkey, run_once) in REGISTRY_SOURCES {
        // The approval records decide the `enabled` flag, so they are read
        // before the entries they describe.
        let approvals = read_approval_map(hive, &format!("{}{}", APPROVAL_PREFIX, approval_subkey));
        scan_registry_run(hive, subkey, label, run_once, &approvals, &mut items);
    }

    for (env_var, label, hive) in [
        ("APPDATA", SOURCE_FOLDER_USER, HKEY_CURRENT_USER),
        ("ProgramData", SOURCE_FOLDER_COMMON, HKEY_LOCAL_MACHINE),
    ] {
        let Some(root) = std::env::var_os(env_var) else {
            continue;
        };
        let folder = PathBuf::from(root).join("Microsoft\\Windows\\Start Menu\\Programs\\Startup");
        let approvals = read_approval_map(hive, &format!("{}StartupFolder", APPROVAL_PREFIX));
        scan_folder(&folder, label, &approvals, &mut items);
    }

    scan_scheduled_startup_tasks(&mut items);
    scan_winlogon(&mut items);

    items
}

// ---------------------------------------------------------------- registry --

/// Call `visit` for every value under `hive\subkey`, then close the key.
///
/// Both registry walks share this shape. One rule is stated once here: a value
/// that does not fit the buffer (`ERROR_MORE_DATA`) is skipped and the walk
/// continues, because aborting would silently drop every entry after it.
fn visit_registry_values(hive: HKEY, subkey: &str, mut visit: impl FnMut(&str, u32, &[u8])) {
    const NAME_CHARS: usize = 512;
    const DATA_BYTES: usize = 4096;

    let subkey_wide: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
    let mut hkey = HKEY::default();

    // SAFETY: `hkey` is a valid out-parameter for the call and the wide subkey
    // buffer outlives it. Every buffer handed to the API below is owned by this
    // frame and its length is passed alongside it.
    unsafe {
        if RegOpenKeyExW(hive, PCWSTR(subkey_wide.as_ptr()), 0, KEY_READ, &mut hkey)
            != ERROR_SUCCESS
        {
            return;
        }

        let mut index = 0u32;
        loop {
            let mut name_buf = vec![0u16; NAME_CHARS];
            let mut name_len = NAME_CHARS as u32;
            let mut val_type = 0u32;
            let mut data = vec![0u8; DATA_BYTES];
            let mut data_len = DATA_BYTES as u32;

            let res = RegEnumValueW(
                hkey,
                index,
                windows::core::PWSTR(name_buf.as_mut_ptr()),
                &mut name_len,
                None,
                Some(&mut val_type),
                Some(data.as_mut_ptr()),
                Some(&mut data_len),
            );
            if res == ERROR_MORE_DATA {
                index += 1;
                continue;
            }
            if res != ERROR_SUCCESS {
                break;
            }

            let name = String::from_utf16_lossy(&name_buf[..name_len as usize])
                .trim()
                .to_string();
            let len = (data_len as usize).min(DATA_BYTES);
            visit(&name, val_type, &data[..len]);
            index += 1;
        }

        let _ = RegCloseKey(hkey);
    }
}

fn scan_registry_run(
    hive: HKEY,
    subkey: &str,
    label: &str,
    run_once: bool,
    approvals: &HashMap<String, bool>,
    items: &mut Vec<StartupItem>,
) {
    visit_registry_values(hive, subkey, |name, val_type, data| {
        let command = decode_reg_string(val_type, data);
        if name.is_empty() || command.is_empty() {
            return;
        }

        let exe_path = extract_exe_path(&command);
        let impact = if run_once {
            "Low".to_string()
        } else {
            estimate_startup_impact(name, &command)
        };
        // One verification per entry: the Authenticode check hits the trust
        // provider and is not free.
        let is_signed = signature_of(&exe_path);

        items.push(StartupItem {
            name: name.to_string(),
            command,
            exe_path,
            source: label.to_string(),
            // No approval record means Windows has never seen the entry
            // disabled, so it launches.
            enabled: approval_lookup(approvals, name).unwrap_or(true),
            impact,
            is_signed,
            publisher: publisher_label(is_signed),
        });
    });
}

/// Decode a string-typed registry payload. `REG_EXPAND_SZ` is accepted
/// alongside `REG_SZ` because Windows itself expands those values at launch, so
/// they are ordinary startup commands.
fn decode_reg_string(val_type: u32, data: &[u8]) -> String {
    if val_type != REG_SZ.0 && val_type != REG_EXPAND_SZ.0 {
        return String::new();
    }
    // SAFETY: the API reported an even number of bytes of a UTF-16 payload, so
    // the u16 view covers at most `data`.
    let words = unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u16, data.len() / 2) };
    String::from_utf16_lossy(words)
        .trim_matches('\0')
        .trim()
        .to_string()
}

/// Read a `StartupApproved` key into value-name to enabled.
///
/// Values that are not `REG_BINARY` or whose byte pattern is unrecognized are
/// left out, so the caller falls back to "enabled" rather than inventing a
/// state.
fn read_approval_map(hive: HKEY, subkey: &str) -> HashMap<String, bool> {
    let mut map = HashMap::new();

    visit_registry_values(hive, subkey, |name, val_type, data| {
        if val_type != REG_BINARY.0 || name.is_empty() {
            return;
        }
        if let Some(enabled) = parse_startup_approved(data) {
            map.insert(name.to_string(), enabled);
        }
    });

    map
}

/// Interpret a `StartupApproved` binary record.
///
/// Windows writes a 12-byte record whose first byte carries the state: `0x02`
/// enabled, `0x03` disabled, `0x06` enabled-but-re-approved (the remaining bytes
/// are a FILETIME). Anything else — including an empty value — is `None`: an
/// unrecognized record must not become a confident claim about the entry.
pub fn parse_startup_approved(bytes: &[u8]) -> Option<bool> {
    match bytes.first()? {
        0x02 | 0x06 => Some(true),
        0x03 => Some(false),
        _ => None,
    }
}

/// Case-insensitive lookup: the approval record's value name and the `Run` value
/// name are written by different components and need not agree on case.
fn approval_lookup(map: &HashMap<String, bool>, name: &str) -> Option<bool> {
    if let Some(found) = map.get(name) {
        return Some(*found);
    }
    map.iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| *value)
}

// ------------------------------------------------------------------ folder --

/// Split a Startup-folder file name into its real name and whether Wino has
/// disabled it with the `.disabled` suffix.
pub fn parse_folder_entry(file_name: &str) -> (&str, bool) {
    const SUFFIX: &str = ".disabled";
    if file_name.len() > SUFFIX.len() && file_name.to_ascii_lowercase().ends_with(SUFFIX) {
        return (&file_name[..file_name.len() - SUFFIX.len()], true);
    }
    (file_name, false)
}

fn scan_folder(
    folder: &Path,
    source_name: &str,
    approvals: &HashMap<String, bool>,
    items: &mut Vec<StartupItem>,
) {
    if !folder.is_dir() {
        return;
    }

    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy().to_string();
        let (base_name, renamed_off) = parse_folder_entry(&file_name);
        if base_name.eq_ignore_ascii_case("desktop.ini") {
            continue;
        }

        let display_name = Path::new(base_name)
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or_else(|| base_name.to_string());

        let command = path.to_string_lossy().to_string();
        let is_signed = crate::security::signatures::is_file_signed(&command);
        let impact = estimate_startup_impact(&display_name, &command);
        // The `.disabled` rename is Wino's own marker and wins over the
        // approval record: a renamed file is not launched either way.
        let enabled = !renamed_off
            && approval_lookup(approvals, base_name)
                .or_else(|| approval_lookup(approvals, &display_name))
                .unwrap_or(true);

        items.push(StartupItem {
            name: display_name,
            exe_path: command.clone(),
            command,
            source: source_name.to_string(),
            enabled,
            impact,
            is_signed,
            publisher: publisher_label(is_signed),
        });
    }
}

// ---------------------------------------------------------- scheduled tasks --

/// Walk `%WinDir%\System32\Tasks\Microsoft\Windows` for logon/boot triggers.
///
/// Only that subtree is walked: it is where the user-facing startup tasks live,
/// and restricting the walk keeps the scan off the hundreds of unrelated
/// maintenance definitions elsewhere in the store.
fn scan_scheduled_startup_tasks(items: &mut Vec<StartupItem>) {
    let Some(root) = crate::tasks::scanner::tasks_root() else {
        return;
    };
    let scope = root.join("Microsoft").join("Windows");
    if !scope.is_dir() {
        return;
    }

    let mut budget = TASK_FILE_BUDGET;
    let truncated = walk_tasks(&scope, "Microsoft\\Windows", &mut budget, items);
    if truncated {
        crate::core::logger::log_warn(
            "startup",
            &format!(
                "Scheduled-task startup scan stopped at the {} file budget; some logon tasks were not inspected.",
                TASK_FILE_BUDGET
            ),
        );
    }
}

/// Returns true when the budget ran out before the walk finished.
fn walk_tasks(dir: &Path, rel: &str, budget: &mut usize, out: &mut Vec<StartupItem>) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };

    for entry in entries.flatten() {
        if *budget == 0 {
            return true;
        }

        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let child_rel = format!("{}\\{}", rel, name);

        if path.is_dir() {
            if walk_tasks(&path, &child_rel, budget, out) {
                return true;
            }
        } else {
            *budget -= 1;
            if let Some(item) = startup_task_item(&path, &child_rel) {
                out.push(item);
            }
        }
    }

    false
}

/// Build a startup item for one task definition, or `None` when it is not a
/// logon/boot task that launches a real user-facing executable.
fn startup_task_item(path: &Path, task_rel_path: &str) -> Option<StartupItem> {
    let xml = read_task_xml(path)?;
    if !has_logon_or_boot_trigger(&xml) {
        return None;
    }

    let raw_command = xml_element_text(&xml, "Command")?;
    let command = expand_env(&unescape_xml(raw_command), env_lookup);

    // Task definitions quote paths inconsistently, so the command line is split
    // the same way the uninstaller splitter does it rather than with the
    // first-token rule.
    let exe_path = crate::apps::models::split_command_line(&command)
        .map(|(exe, _)| exe)
        .unwrap_or_default();
    if exe_path.is_empty() || is_under_system32(&exe_path) {
        return None;
    }
    if !Path::new(&exe_path).is_file() {
        return None;
    }

    let is_signed = signature_of(&exe_path);
    Some(StartupItem {
        name: task_rel_path.to_string(),
        command,
        exe_path,
        source: SOURCE_SCHEDULED_TASK.to_string(),
        enabled: xml_element_text(&xml, "Enabled")
            .map(|flag| !flag.eq_ignore_ascii_case("false"))
            .unwrap_or(true),
        impact: "Medium".to_string(),
        is_signed,
        publisher: publisher_label(is_signed),
    })
}

/// Read a task definition. The store writes UTF-16LE with a BOM, which
/// `fs::read_to_string` cannot decode, so the encoding is handled here.
fn read_task_xml(path: &Path) -> Option<String> {
    const MAX_TASK_XML_BYTES: u64 = 1024 * 1024;

    let metadata = fs::metadata(path).ok()?;
    if metadata.len() > MAX_TASK_XML_BYTES {
        return None;
    }

    let bytes = fs::read(path).ok()?;
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        let words: Vec<u16> = bytes[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        return Some(String::from_utf16_lossy(&words));
    }

    String::from_utf8(bytes).ok()
}

/// Whether a task definition triggers at logon or at boot.
pub fn has_logon_or_boot_trigger(xml: &str) -> bool {
    let lower = xml.to_ascii_lowercase();
    lower.contains("<logontrigger") || lower.contains("<boottrigger")
}

/// Text of the first `<tag>` element, or `None` when it is absent.
///
/// Deliberately a scan rather than a parser: task definitions are a known,
/// narrow shape and pulling in an XML dependency for two elements is not worth
/// it. ASCII-lowercasing preserves byte offsets, so the indices found in the
/// lowered copy are valid in the original.
///
/// A longer tag name that merely starts with `tag` (`<CommandLine>` for
/// `<Command>`) is skipped, not treated as the end of the search.
pub fn xml_element_text<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let lower = xml.to_ascii_lowercase();
    let open = format!("<{}", tag.to_ascii_lowercase());
    let close = format!("</{}>", tag.to_ascii_lowercase());

    let mut search_from = 0usize;
    while let Some(found) = lower[search_from..].find(&open) {
        let start = search_from + found;
        search_from = start + open.len();

        let Some(after_open) = xml[search_from..].chars().next() else {
            break;
        };
        if after_open != '>' && !after_open.is_whitespace() {
            continue;
        }

        let body = &xml[search_from..];
        let Some(body_start) = body.find('>') else {
            break;
        };
        let rest = &body[body_start + 1..];
        let end = rest.to_ascii_lowercase().find(&close)?;
        return Some(rest[..end].trim());
    }

    None
}

/// Resolve the five predefined XML entities. `&amp;` is replaced last so a
/// literal `&amp;lt;` cannot turn into `<`.
pub fn unescape_xml(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// Expand `%VAR%` references using `lookup`, leaving anything unknown intact.
///
/// Task commands frequently contain `%windir%` or `%ProgramFiles%`, which must
/// be resolved before the executable can be checked on disk. One pass over the
/// text: an unmatched or unknown `%NAME%` is copied through unchanged rather
/// than dropped, so a command is never silently rewritten into a different one.
pub fn expand_env(text: &str, lookup: impl Fn(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(open) = rest.find('%') {
        out.push_str(&rest[..open]);
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find('%') else {
            // A lone '%' at the end: not a reference.
            rest = &rest[open..];
            break;
        };

        let name = &after_open[..close];
        match lookup(name) {
            Some(value) => out.push_str(&value),
            None => out.push_str(&rest[open..open + 1 + close + 1]),
        }
        rest = &after_open[close + 1..];
    }

    out.push_str(rest);
    out
}

/// Environment lookup for the names task commands actually reference.
fn env_lookup(name: &str) -> Option<String> {
    let value = std::env::var(name).ok()?;
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

/// `%WinDir%\System32`, or `None` when the environment does not say.
fn system32_dir() -> Option<String> {
    let windir = std::env::var("WinDir").ok().filter(|d| !d.is_empty())?;
    Some(format!("{}\\System32", windir.trim_end_matches('\\')))
}

/// Whether an executable path lives under `%WinDir%\System32`.
///
/// System tasks are not user startup applications, so they are excluded from the
/// scheduled-task results. The separator after `System32` is required so a
/// sibling directory sharing the prefix is not mistaken for it.
pub fn is_under_system32(exe_path: &str) -> bool {
    let Some(system32) = system32_dir() else {
        return false;
    };
    let prefix = format!("{}\\", system32);
    exe_path
        .to_ascii_lowercase()
        .starts_with(&prefix.to_ascii_lowercase())
}

// ----------------------------------------------------------------- winlogon --

/// Report the `Winlogon` `Shell` and `Userinit` values.
///
/// Security-sensitive: anything written there runs as the signed-in user before
/// the desktop appears. Reported with High impact and never toggled —
/// `startup::manager` refuses the source.
fn scan_winlogon(items: &mut Vec<StartupItem>) {
    for value_name in ["Shell", "Userinit"] {
        let Some(command) =
            crate::core::regutil::read_string(HKEY_LOCAL_MACHINE, WINLOGON_KEY, value_name)
        else {
            continue;
        };
        if command.trim().is_empty() {
            continue;
        }

        // The image is named bare, so it has to be resolved before the
        // signature check or every default entry reads as unverified.
        let exe_path = resolve_bare_executable(
            &crate::apps::models::split_command_line(&command)
                .map(|(exe, _)| exe)
                .unwrap_or_default(),
        );
        let is_signed = signature_of(&exe_path);

        items.push(StartupItem {
            name: value_name.to_string(),
            command,
            exe_path,
            source: SOURCE_WINLOGON.to_string(),
            enabled: true,
            impact: "High".to_string(),
            is_signed,
            publisher: publisher_label(is_signed),
        });
    }
}

/// Resolve a bare image name the way Windows does.
///
/// Winlogon names its image without a directory, and those do not all live in
/// one place: `explorer.exe` is in `%WinDir%`, `userinit.exe` in `System32`.
/// `SearchPathW` is given exactly those two directories rather than being
/// allowed to fall back to the current directory, so the result cannot be
/// steered by where Wino was started from.
fn resolve_bare_executable(exe_path: &str) -> String {
    if exe_path.is_empty() || exe_path.contains('\\') || exe_path.contains('/') {
        return exe_path.to_string();
    }
    let Some(windir) = std::env::var("WinDir").ok().filter(|d| !d.is_empty()) else {
        return exe_path.to_string();
    };
    let windir = windir.trim_end_matches('\\');

    let search_dirs: Vec<u16> = format!("{}\\System32;{}", windir, windir)
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let file: Vec<u16> = exe_path.encode_utf16().chain(std::iter::once(0)).collect();
    let mut buffer = vec![0u16; 512];

    // SAFETY: both wide strings are NUL-terminated and outlive the call, and
    // `buffer` is a valid destination of the length passed to the API.
    let written = unsafe {
        windows::Win32::Storage::FileSystem::SearchPathW(
            PCWSTR(search_dirs.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR::null(),
            Some(&mut buffer),
            None,
        )
    };

    if written == 0 || written as usize > buffer.len() {
        return exe_path.to_string();
    }

    String::from_utf16_lossy(&buffer[..written as usize])
        .trim_matches('\0')
        .to_string()
}

// ------------------------------------------------------------------ helpers --

/// First token of a command line, honouring quotes. Kept as-is: the impact
/// estimate and the UI both depend on this shape.
fn extract_exe_path(command: &str) -> String {
    let trimmed = command.trim();
    if let Some(stripped) = trimmed.strip_prefix('"') {
        if let Some(end_quote) = stripped.find('"') {
            return stripped[..end_quote].to_string();
        }
    }
    trimmed.split_whitespace().next().unwrap_or("").to_string()
}

fn estimate_startup_impact(name: &str, command: &str) -> String {
    let lower = format!("{} {}", name, command).to_lowercase();
    if lower.contains("discord")
        || lower.contains("steam")
        || lower.contains("spotify")
        || lower.contains("teams")
        || lower.contains("slack")
    {
        "High".to_string()
    } else if lower.contains("update")
        || lower.contains("helper")
        || lower.contains("cloud")
        || lower.contains("onedrive")
    {
        "Medium".to_string()
    } else {
        "Low".to_string()
    }
}

fn signature_of(exe_path: &str) -> bool {
    if exe_path.is_empty() {
        return false;
    }
    crate::security::signatures::is_file_signed(exe_path)
}

fn publisher_label(is_signed: bool) -> String {
    if is_signed {
        "Verified".to_string()
    } else {
        "Unverified".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approval_patterns_from_real_windows_records() {
        // 0x02 enabled and 0x03 disabled are what Windows writes on the machine
        // this was developed against; 0x06 is a re-approved enabled entry.
        assert_eq!(
            parse_startup_approved(&[0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            Some(true)
        );
        assert_eq!(
            parse_startup_approved(&[0x03, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            Some(false)
        );
        assert_eq!(
            parse_startup_approved(&[
                0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
            ]),
            Some(true)
        );

        // Anything unrecognized reports unknown, never a confident state.
        assert_eq!(parse_startup_approved(&[]), None);
        assert_eq!(parse_startup_approved(&[0x00]), None);
        assert_eq!(parse_startup_approved(&[0x04, 0x11, 0x22]), None);
        assert_eq!(parse_startup_approved(&[0xFF; 12]), None);
    }

    #[test]
    fn folder_suffix_marks_disabled_entries_and_preserves_extensions() {
        assert_eq!(parse_folder_entry("Spotify.lnk"), ("Spotify.lnk", false));
        assert_eq!(
            parse_folder_entry("Spotify.lnk.disabled"),
            ("Spotify.lnk", true)
        );
        assert_eq!(parse_folder_entry("tool.exe.disabled"), ("tool.exe", true));
        assert_eq!(
            parse_folder_entry("DISABLED.LNK.DISABLED"),
            ("DISABLED.LNK", true)
        );
        assert_eq!(parse_folder_entry("notes.txt"), ("notes.txt", false));
        // A file whose whole name is the suffix is not a disabled entry.
        assert_eq!(parse_folder_entry(".disabled"), (".disabled", false));

        // The display name drops the suffix and the real extension.
        let (base, _) = parse_folder_entry("My Tool.lnk.disabled");
        let stem = Path::new(base)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string());
        assert_eq!(stem.as_deref(), Some("My Tool"));
    }

    #[test]
    fn trigger_detection_and_command_extraction() {
        let logon = r#"<Task><Triggers><LogonTrigger><Enabled>true</Enabled></LogonTrigger></Triggers></Task>"#;
        let boot = r#"<Task><Triggers><BootTrigger/></Triggers></Task>"#;
        let time = r#"<Task><Triggers><TimeTrigger><StartBoundary>2026-01-01T00:00:00</StartBoundary></TimeTrigger></Triggers></Task>"#;
        assert!(has_logon_or_boot_trigger(logon));
        assert!(has_logon_or_boot_trigger(boot));
        assert!(!has_logon_or_boot_trigger(time));
        assert!(!has_logon_or_boot_trigger(""));

        let xml = r#"<Task><Actions><Exec><Command>C:\Tools\app.exe</Command>
        <Arguments>--start &amp; --quiet</Arguments></Exec></Actions></Task>"#;
        assert_eq!(xml_element_text(xml, "Command"), Some(r"C:\Tools\app.exe"));
        let args = xml_element_text(xml, "Arguments").expect("arguments element");
        assert_eq!(unescape_xml(args), "--start & --quiet");
        assert_eq!(xml_element_text(xml, "WorkingDirectory"), None);
    }

    #[test]
    fn element_scan_does_not_match_a_longer_tag_name() {
        let xml = "<CommandLine>other</CommandLine><Command>real.exe</Command>";
        assert_eq!(xml_element_text(xml, "Command"), Some("real.exe"));
        assert_eq!(
            xml_element_text("<NoCommand>x</NoCommand>", "Command"),
            None
        );
        // An unterminated element reports absent rather than swallowing the rest.
        assert_eq!(xml_element_text("<Command>never closed", "Command"), None);
        assert_eq!(xml_element_text("", "Command"), None);
    }

    #[test]
    fn environment_expansion_resolves_known_names_and_leaves_unknown_ones() {
        let lookup = |name: &str| match name.to_ascii_lowercase().as_str() {
            "windir" => Some(r"C:\Windows".to_string()),
            _ => None,
        };

        assert_eq!(
            expand_env(r"%WINDIR%\System32\a.exe", lookup),
            r"C:\Windows\System32\a.exe"
        );
        assert_eq!(expand_env(r"%windir%\b.exe", lookup), r"C:\Windows\b.exe");
        // An unknown name is copied through, never dropped.
        assert_eq!(expand_env(r"%nope%\c.exe", lookup), r"%nope%\c.exe");
        assert_eq!(expand_env("plain.exe", lookup), "plain.exe");
        // A trailing '%' is not a reference.
        assert_eq!(expand_env("50% done", lookup), "50% done");
        assert_eq!(expand_env("%windir", lookup), "%windir");
        assert_eq!(expand_env("", lookup), "");
        // Two references in one command.
        assert_eq!(
            expand_env(r"%windir%\a.exe --root %windir%", lookup),
            r"C:\Windows\a.exe --root C:\Windows"
        );
    }

    #[test]
    fn system32_filter_rejects_windows_own_tasks() {
        // Guarded on the real environment: on a machine without %WinDir% the
        // filter cannot judge anything, which is why it must return false there.
        let Some(windir) = std::env::var("WinDir").ok().filter(|d| !d.is_empty()) else {
            assert!(!is_under_system32("anything.exe"));
            return;
        };

        let inside = format!("{}\\System32\\defrag.exe", windir);
        let outside = r"C:\Program Files\Vendor\updater.exe";
        assert!(is_under_system32(&inside));
        assert!(is_under_system32(&inside.to_uppercase()));
        assert!(!is_under_system32(outside));
        // A sibling directory sharing the prefix is not System32 itself.
        assert!(!is_under_system32(&format!(
            "{}\\System32Extra\\x.exe",
            windir
        )));
        // System32 itself, with no trailing file, still counts.
        assert!(is_under_system32(&format!("{}\\System32\\x.exe", windir)));
    }

    #[test]
    fn registry_sources_map_to_their_own_subkey_and_read_only_sources_have_none() {
        assert_eq!(
            registry_source_target(SOURCE_HKCU_RUN),
            Some(("HKCU", RUN_KEY))
        );
        assert_eq!(
            registry_source_target(SOURCE_HKLM_WOW64_RUN),
            Some(("HKLM", RUN_WOW64_KEY))
        );
        assert_eq!(
            registry_source_target(SOURCE_HKLM_RUN_ONCE),
            Some(("HKLM", RUN_ONCE_KEY))
        );
        assert_eq!(
            registry_source_target(SOURCE_HKCU_RUN_ONCE),
            Some(("HKCU", RUN_ONCE_KEY))
        );

        // The RunOnce subkeys must not be confused with Run.
        assert_ne!(
            registry_source_target(SOURCE_HKLM_RUN_ONCE).map(|t| t.1),
            Some(RUN_KEY)
        );

        assert!(registry_source_target(SOURCE_WINLOGON).is_none());
        assert!(registry_source_target(SOURCE_SCHEDULED_TASK).is_none());
        assert!(registry_source_target(SOURCE_FOLDER_USER).is_none());
    }

    #[test]
    fn approval_lookup_ignores_case() {
        let mut map = HashMap::new();
        map.insert("Spotify".to_string(), false);
        assert_eq!(approval_lookup(&map, "Spotify"), Some(false));
        assert_eq!(approval_lookup(&map, "spotify"), Some(false));
        assert_eq!(approval_lookup(&map, "Steam"), None);
    }

    #[test]
    fn bare_winlogon_image_names_resolve_to_the_real_file() {
        // `explorer.exe` sits in %WinDir% and `userinit.exe` in System32, so a
        // single-directory guess would miss one of them. Both are checked when
        // the environment allows it; the resolver must at worst return its input
        // unchanged, never a path to a file that is not there.
        let Some(windir) = std::env::var("WinDir").ok().filter(|d| !d.is_empty()) else {
            assert_eq!(resolve_bare_executable("explorer.exe"), "explorer.exe");
            return;
        };
        let _ = windir;

        let resolved = resolve_bare_executable("explorer.exe");
        assert!(
            Path::new(&resolved).is_file(),
            "explorer.exe did not resolve to a file: {}",
            resolved
        );

        let userinit = resolve_bare_executable("userinit.exe");
        assert!(
            Path::new(&userinit).is_file(),
            "userinit.exe did not resolve to a file: {}",
            userinit
        );

        // A path is left alone, and an unknown name falls back to itself.
        assert_eq!(
            resolve_bare_executable(r"C:\Other\a.exe"),
            r"C:\Other\a.exe"
        );
        assert_eq!(resolve_bare_executable(""), "");
        assert_eq!(
            resolve_bare_executable("no-such-image-xyz.exe"),
            "no-such-image-xyz.exe"
        );
    }

    #[test]
    fn exe_extraction_keeps_the_legacy_first_token_behaviour() {
        assert_eq!(
            extract_exe_path(r#""C:\Program Files\App\app.exe" -flag"#),
            r"C:\Program Files\App\app.exe"
        );
        assert_eq!(extract_exe_path(r"C:\App\app.exe -flag"), r"C:\App\app.exe");
        assert_eq!(extract_exe_path("   "), "");
    }

    #[test]
    fn impact_estimation_keeps_its_buckets() {
        assert_eq!(estimate_startup_impact("Discord", ""), "High");
        assert_eq!(estimate_startup_impact("OneDrive", ""), "Medium");
        assert_eq!(
            estimate_startup_impact("Vendor Tool", r"C:\x\tool.exe"),
            "Low"
        );
    }
}
