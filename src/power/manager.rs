//! Power Manager: native enumeration and switching of Windows power schemes.
//!
//! Everything here goes through the `powrprof.dll` power API instead of
//! `powercfg.exe`, with exactly one documented exception: creating the Ultimate
//! Performance scheme (see [`ensure_ultimate_performance`]).
//!
//! Scheme GUIDs travel as uppercase, brace-free strings because that is the form
//! [`crate::power::models::scheme_guids`] uses and the form the UI displays.

use crate::core::logger::{log_error, log_info};
use crate::core::safety::{OperationDescriptor, RiskLevel, SafetyEngine};
use crate::core::system::SystemInfo;
use crate::power::models::{scheme_guids, PowerPlanKind};
// Re-exported because the UI imports `PowerPlan` from this module.
pub use crate::power::models::PowerPlan;
use crate::restore::snapshots::create_snapshot;
use windows::core::GUID;
use windows::Win32::Foundation::{
    LocalFree, ERROR_ACCESS_DENIED, ERROR_NO_MORE_ITEMS, ERROR_SUCCESS, HLOCAL,
};
use windows::Win32::System::Power::{
    PowerEnumerate, PowerGetActiveScheme, PowerReadFriendlyName, PowerSetActiveScheme,
    ACCESS_SCHEME,
};
use windows::Win32::System::Registry::HKEY;

/// Template scheme that Ultimate Performance is duplicated from. Windows ships
/// the definition but hides it until it is duplicated.
const ULTIMATE_TEMPLATE_GUID: &str = "e9a42b02-d5df-448d-aa00-03f14749eb61";

/// Render a GUID as uppercase hex without braces, the canonical Wino form.
pub fn guid_to_string(guid: &GUID) -> String {
    format!(
        "{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        guid.data1,
        guid.data2,
        guid.data3,
        guid.data4[0],
        guid.data4[1],
        guid.data4[2],
        guid.data4[3],
        guid.data4[4],
        guid.data4[5],
        guid.data4[6],
        guid.data4[7]
    )
}

/// Parse a GUID string (`{...}` braces optional, any case).
///
/// Written by hand instead of `GUID::from(&str)` because that conversion
/// panics on malformed input, and user-supplied ids must not be able to crash
/// the app.
pub fn parse_guid(text: &str) -> Option<GUID> {
    let trimmed = text.trim().trim_start_matches('{').trim_end_matches('}');
    if !trimmed.is_ascii() {
        return None;
    }
    let parts: Vec<&str> = trimmed.split('-').collect();
    if parts.len() != 5 {
        return None;
    }
    let expected = [8usize, 4, 4, 4, 12];
    if parts.iter().zip(expected).any(|(p, len)| p.len() != len) {
        return None;
    }

    let data1 = u32::from_str_radix(parts[0], 16).ok()?;
    let data2 = u16::from_str_radix(parts[1], 16).ok()?;
    let data3 = u16::from_str_radix(parts[2], 16).ok()?;

    let mut data4 = [0u8; 8];
    let group3 = u16::from_str_radix(parts[3], 16).ok()?;
    data4[0] = (group3 >> 8) as u8;
    data4[1] = (group3 & 0xFF) as u8;
    for i in 0..6 {
        data4[2 + i] = u8::from_str_radix(&parts[4][i * 2..i * 2 + 2], 16).ok()?;
    }

    Some(GUID::from_values(data1, data2, data3, data4))
}

/// Resolve a plan alias to its well-known scheme GUID.
pub fn resolve_plan_alias(id: &str) -> Option<&'static str> {
    // Accept both spellings: "high_performance" and "high-performance".
    let normalized = id.trim().to_lowercase().replace('-', "_");
    match normalized.as_str() {
        "balanced" => Some(scheme_guids::BALANCED),
        "high_performance" => Some(scheme_guids::HIGH_PERFORMANCE),
        "ultimate" | "ultimate_performance" => Some(scheme_guids::ULTIMATE_PERFORMANCE),
        "power_saver" => Some(scheme_guids::POWER_SAVER),
        _ => None,
    }
}

/// Resolve an alias or GUID string to the canonical uppercase GUID string.
pub fn resolve_plan_id(id: &str) -> Option<String> {
    if let Some(alias) = resolve_plan_alias(id) {
        return Some(alias.to_string());
    }
    parse_guid(id).map(|guid| guid_to_string(&guid))
}

/// Pull the first scheme GUID out of tool output.
///
/// `powercfg` is localized, so the surrounding words cannot be matched; the
/// GUID itself is locale-independent.
pub fn parse_guid_from_output(output: &str) -> Option<String> {
    for token in output.split_whitespace() {
        let cleaned = token.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-');
        if cleaned.len() == 36 {
            if let Some(guid) = parse_guid(cleaned) {
                return Some(guid_to_string(&guid));
            }
        }
    }
    None
}

/// Read a scheme's display name. Windows localizes it, so it is never
/// hard-coded on our side.
fn read_friendly_name(guid: &GUID) -> String {
    let mut size = 0u32;
    unsafe {
        let _ = PowerReadFriendlyName(
            HKEY::default(),
            Some(guid as *const GUID),
            None,
            None,
            None,
            &mut size,
        );
    }
    if size == 0 {
        return String::new();
    }

    let mut buffer = vec![0u8; size as usize];
    let result = unsafe {
        PowerReadFriendlyName(
            HKEY::default(),
            Some(guid as *const GUID),
            None,
            None,
            Some(buffer.as_mut_ptr()),
            &mut size,
        )
    };
    if result != ERROR_SUCCESS {
        return String::new();
    }

    let units: Vec<u16> = buffer
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_ne_bytes([pair[0], pair[1]]))
        .collect();
    String::from_utf16_lossy(&units)
        .trim_matches('\0')
        .trim()
        .to_string()
}

/// Enumerate every power scheme registered on this machine.
pub fn list_plans() -> Vec<PowerPlan> {
    let active = active_plan_guid();
    let mut plans = Vec::new();
    let mut index = 0u32;

    loop {
        let mut size = 0u32;
        let probe = unsafe {
            PowerEnumerate(
                HKEY::default(),
                None,
                None,
                ACCESS_SCHEME,
                index,
                None,
                &mut size,
            )
        };
        if probe == ERROR_NO_MORE_ITEMS || size == 0 {
            break;
        }

        let mut buffer = vec![0u8; size as usize];
        let result = unsafe {
            PowerEnumerate(
                HKEY::default(),
                None,
                None,
                ACCESS_SCHEME,
                index,
                Some(buffer.as_mut_ptr()),
                &mut size,
            )
        };
        if result != ERROR_SUCCESS || size < 16 {
            break;
        }

        // SAFETY: the API filled `buffer` with one GUID (16 bytes, checked
        // above). `read_unaligned` because a Vec<u8> makes no alignment
        // promise beyond 1 byte.
        let guid = unsafe { std::ptr::read_unaligned(buffer.as_ptr() as *const GUID) };
        let guid_string = guid_to_string(&guid);

        plans.push(PowerPlan {
            is_active: active
                .as_deref()
                .map(|a| a.eq_ignore_ascii_case(&guid_string))
                .unwrap_or(false),
            kind: PowerPlanKind::from_guid(&guid_string),
            name: read_friendly_name(&guid),
            guid: guid_string,
        });

        index += 1;
    }

    plans.sort_by(|a, b| {
        b.is_active
            .cmp(&a.is_active)
            .then(kind_rank(a.kind).cmp(&kind_rank(b.kind)))
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    plans
}

/// Display order of the known plan kinds.
fn kind_rank(kind: PowerPlanKind) -> u8 {
    match kind {
        PowerPlanKind::Balanced => 0,
        PowerPlanKind::HighPerformance => 1,
        PowerPlanKind::PowerSaver => 2,
        PowerPlanKind::Ultimate => 3,
        PowerPlanKind::Custom => 4,
    }
}

/// GUID of the currently active scheme, uppercase without braces.
pub fn active_plan_guid() -> Option<String> {
    let mut raw: *mut GUID = std::ptr::null_mut();
    let result = unsafe { PowerGetActiveScheme(HKEY::default(), &mut raw) };
    if result != ERROR_SUCCESS || raw.is_null() {
        return None;
    }

    // SAFETY: on success the API returned a LocalAlloc'd GUID that this process
    // owns. Read it, then free it exactly once — leaking it is a bug.
    let guid = unsafe { std::ptr::read_unaligned(raw) };
    unsafe {
        let _ = LocalFree(HLOCAL(raw as *mut core::ffi::c_void));
    }

    Some(guid_to_string(&guid))
}

/// Switch the active power plan. `plan_id` is a scheme GUID or one of the
/// aliases `balanced`, `high_performance`, `ultimate`, `power_saver`.
///
/// A plan that is not installed is an error, never a silent no-op: switching to
/// Ultimate Performance requires creating it first.
pub fn set_active_plan(plan_id: &str, dry_run: bool) -> Result<String, String> {
    let Some(guid_string) = resolve_plan_id(plan_id) else {
        return Err(format!(
            "'{}' is not a power plan. Use a scheme GUID or one of: balanced, high_performance, ultimate, power_saver.",
            plan_id
        ));
    };

    let plans = list_plans();
    let Some(plan) = plans
        .iter()
        .find(|p| p.guid.eq_ignore_ascii_case(&guid_string))
    else {
        let hint = if guid_string == scheme_guids::ULTIMATE_PERFORMANCE {
            " Ultimate Performance is not installed by default; create it first with ensure_ultimate_performance()."
        } else {
            ""
        };
        return Err(format!(
            "Power plan '{}' is not present on this system.{}",
            plan_id, hint
        ));
    };

    if dry_run {
        return Ok(format!(
            "[DRY-RUN] Would switch to power plan '{}' ({}).",
            plan.name, plan.guid
        ));
    }

    let Some(guid) = parse_guid(&plan.guid) else {
        return Err(format!("Scheme GUID '{}' could not be parsed.", plan.guid));
    };

    let _ = create_snapshot(&format!("Power plan: {}", plan.name));

    let result = unsafe { PowerSetActiveScheme(HKEY::default(), Some(&guid as *const GUID)) };
    if result == ERROR_ACCESS_DENIED {
        return Err(format!(
            "Switching the power plan requires Administrator privileges (Windows denied the request for '{}').",
            plan.name
        ));
    }
    if result != ERROR_SUCCESS {
        log_error(
            "power",
            &format!(
                "PowerSetActiveScheme failed for {} ({})",
                plan.name, result.0
            ),
        );
        return Err(format!(
            "Windows refused to activate '{}' (Win32 error {}).",
            plan.name, result.0
        ));
    }

    let message = format!("Active power plan is now '{}' ({}).", plan.name, plan.guid);
    log_info("power", &message);
    Ok(message)
}

/// Make Ultimate Performance available, creating it on first use.
///
/// Windows ships the Ultimate Performance definition but does not register it.
/// There is no native duplicate-scheme entry point in the `windows` crate's
/// exposed surface (`PowerDuplicateScheme` is not exported), so this one
/// operation shells out to `powercfg.exe`, which is the documented interface.
/// Only ever called when the user explicitly asks for Ultimate Performance.
///
/// Returns the GUID of the scheme; activating it is [`set_active_plan`]'s job.
pub fn ensure_ultimate_performance() -> Result<String, String> {
    if let Some(existing) = list_plans()
        .into_iter()
        .find(|plan| plan.kind == PowerPlanKind::Ultimate)
    {
        return Ok(format!(
            "Ultimate Performance is already available as '{}' ({}).",
            existing.name, existing.guid
        ));
    }

    let sys_info = SystemInfo::detect();
    let descriptor = OperationDescriptor::simple(
        "power_create_ultimate",
        "Create Ultimate Performance power plan",
        "Power: Ultimate Performance",
        RiskLevel::Low,
    )
    .with_reason("Duplicates the hidden Ultimate Performance scheme definition Windows ships.")
    .with_admin(true)
    .with_windows(&["10", "11"]);
    let check = SafetyEngine::validate_descriptor(&descriptor, &sys_info);
    if !check.is_allowed {
        return Err(check.reason);
    }

    let output = crate::core::proc::run(
        "powercfg.exe",
        &["-duplicatescheme", ULTIMATE_TEMPLATE_GUID],
    )
    .map_err(|e| format!("Failed to run powercfg.exe: {}", e))?;

    if !output.success {
        log_error(
            "power",
            &format!("powercfg -duplicatescheme failed: {}", output.last_line()),
        );
        return Err(format!(
            "powercfg -duplicatescheme failed (exit {}): {}",
            output
                .exit_code
                .map(|c| c.to_string())
                .unwrap_or_else(|| "none".to_string()),
            output.last_line()
        ));
    }

    // The duplicated scheme gets a fresh GUID, which powercfg prints.
    let Some(created) = parse_guid_from_output(&output.stdout)
        .or_else(|| parse_guid_from_output(&output.combined()))
    else {
        return Err(format!(
            "powercfg reported success but printed no scheme GUID: {}",
            output.last_line()
        ));
    };

    let message = format!(
        "Ultimate Performance created as {}. Select it to make it the active plan.",
        created
    );
    log_info("power", &message);
    Ok(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guid_to_string_formats_uppercase_without_braces() {
        let balanced = GUID::from_u128(0x381b4222_f694_41f0_9685_ff5bb260df2e);
        assert_eq!(
            guid_to_string(&balanced),
            "381B4222-F694-41F0-9685-FF5BB260DF2E"
        );

        let ultimate = GUID::from_u128(0xe9a42b02_d5df_448d_aa00_03f14749eb61);
        assert_eq!(
            guid_to_string(&ultimate),
            scheme_guids::ULTIMATE_PERFORMANCE
        );
        assert_eq!(
            PowerPlanKind::from_guid(&guid_to_string(&ultimate)),
            PowerPlanKind::Ultimate
        );
    }

    #[test]
    fn parse_guid_round_trips_and_rejects_malformed_input() {
        let original = GUID::from_u128(0x8c5e7fda_e8bf_4a96_9a85_a6e23a8c635c);
        let text = guid_to_string(&original);
        let parsed = parse_guid(&text).expect("round trip");
        assert_eq!(parsed, original);
        assert_eq!(guid_to_string(&parsed), scheme_guids::HIGH_PERFORMANCE);

        // Braces and lower case are accepted.
        let braced = parse_guid("{8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c}").expect("braced guid");
        assert_eq!(braced, original);

        for bad in [
            "",
            "balanced",
            "8c5e7fda-e8bf-4a96-9a85",
            "zzzzzzzz-e8bf-4a96-9a85-a6e23a8c635c",
            "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c-extra",
        ] {
            assert!(
                parse_guid(bad).is_none(),
                "expected '{}' to be rejected",
                bad
            );
        }
    }

    #[test]
    fn aliases_resolve_to_well_known_schemes() {
        assert_eq!(resolve_plan_alias("balanced"), Some(scheme_guids::BALANCED));
        assert_eq!(
            resolve_plan_alias("High-Performance"),
            Some(scheme_guids::HIGH_PERFORMANCE)
        );
        assert_eq!(
            resolve_plan_alias("high_performance"),
            Some(scheme_guids::HIGH_PERFORMANCE)
        );
        assert_eq!(
            resolve_plan_alias("ultimate"),
            Some(scheme_guids::ULTIMATE_PERFORMANCE)
        );
        assert_eq!(
            resolve_plan_alias("power_saver"),
            Some(scheme_guids::POWER_SAVER)
        );
        assert!(resolve_plan_alias("turbo").is_none());

        assert_eq!(
            resolve_plan_id("power-saver").as_deref(),
            Some(scheme_guids::POWER_SAVER)
        );
        assert_eq!(
            resolve_plan_id("381b4222-f694-41f0-9685-ff5bb260df2e").as_deref(),
            Some(scheme_guids::BALANCED)
        );
        assert!(resolve_plan_id("not a plan").is_none());
    }

    #[test]
    fn powercfg_output_guid_is_parsed_regardless_of_locale() {
        let english =
            "Power Scheme GUID: e9a42b02-d5df-448d-aa00-03f14749eb61  (Ultimate Performance)";
        assert_eq!(
            parse_guid_from_output(english).as_deref(),
            Some(scheme_guids::ULTIMATE_PERFORMANCE)
        );

        let localized =
            "GUID Skema Daya: e9a42b02-d5df-448d-aa00-03f14749eb61  (Ultimate Performance)";
        assert_eq!(
            parse_guid_from_output(localized).as_deref(),
            Some(scheme_guids::ULTIMATE_PERFORMANCE)
        );

        assert!(parse_guid_from_output("Unable to perform operation.").is_none());
    }

    #[test]
    fn plan_ranking_puts_balanced_before_custom() {
        assert!(kind_rank(PowerPlanKind::Balanced) < kind_rank(PowerPlanKind::HighPerformance));
        assert!(kind_rank(PowerPlanKind::HighPerformance) < kind_rank(PowerPlanKind::PowerSaver));
        assert!(kind_rank(PowerPlanKind::PowerSaver) < kind_rank(PowerPlanKind::Ultimate));
        assert!(kind_rank(PowerPlanKind::Ultimate) < kind_rank(PowerPlanKind::Custom));
    }

    #[test]
    fn unknown_plan_id_is_rejected_before_touching_the_system() {
        // The id cannot be resolved, so the error names the accepted inputs and
        // no Win32 call is made — safe to run on any machine.
        let err = set_active_plan("turbo", true).unwrap_err();
        assert!(
            err.contains("turbo"),
            "error must name the rejected id: {}",
            err
        );
        assert!(
            err.contains("balanced"),
            "error must list the aliases: {}",
            err
        );
    }
}
