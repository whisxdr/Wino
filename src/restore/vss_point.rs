//! Native Windows System Restore Point creation via `SRSetRestorePointW`
//! (SrClient.dll / VSS).
//!
//! The API is resolved at runtime with LoadLibraryW + GetProcAddress so the
//! crate links identically under MSVC and MinGW-GNU without an import library,
//! and with zero PowerShell/.NET involvement.

use crate::core::logger::{log_info, log_warn};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW};
use windows::Win32::System::Registry::{
    RegCloseKey, RegQueryValueExW, RegSetValueExW, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE,
    REG_DWORD,
};

/// RESTOREPOINTINFOW (srclient.h) — manually mirrored layout.
#[repr(C)]
struct RestorePointInfoW {
    dw_event_type: u32,
    dw_restore_pt_type: u32,
    ll_sequence_number: i64,
    sz_description: [u16; 256],
}

/// STATEMGRSTATUS (srclient.h) — manually mirrored layout.
#[repr(C)]
struct StateMgrStatus {
    dw_status: u32,
    ll_sequence_number: i64,
}

type SrSetRestorePointFn =
    unsafe extern "system" fn(*mut RestorePointInfoW, *mut StateMgrStatus) -> i32;

const BEGIN_SYSTEM_CHANGE: u32 = 100;
const END_SYSTEM_CHANGE: u32 = 101;
const MODIFY_SETTINGS: u32 = 3;
/// ERROR_SERVICE_DISABLED — System Protection is switched off.
const ERROR_SERVICE_DISABLED: u32 = 1058;
/// ERROR_SERVICE_NOT_ACTIVE — alternate signature of the same condition.
const ERROR_SERVICE_NOT_ACTIVE: u32 = 1062;

fn resolve_srclient() -> Option<SrSetRestorePointFn> {
    unsafe {
        let module = match GetModuleHandleW(w!("srclient.dll")) {
            Ok(h) => h,
            Err(_) => LoadLibraryW(w!("srclient.dll")).ok()?,
        };
        let proc = GetProcAddress(module, windows::core::s!("SRSetRestorePointW"))?;
        Some(std::mem::transmute::<
            unsafe extern "system" fn() -> isize,
            SrSetRestorePointFn,
        >(proc))
    }
}

fn wide_description(desc: &str) -> [u16; 256] {
    let mut buf = [0u16; 256];
    for (i, unit) in desc.encode_utf16().take(255).enumerate() {
        buf[i] = unit;
    }
    buf
}

/// Read the restore-point creation frequency throttle (minutes). `None` when unset.
fn read_creation_frequency() -> Option<u32> {
    const FREQ_PATH: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\SystemRestore";
    let path_w: Vec<u16> = FREQ_PATH.encode_utf16().chain(std::iter::once(0)).collect();
    let name_w: Vec<u16> = "SystemRestorePointCreationFrequency"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut key = HKEY_LOCAL_MACHINE;
    let mut value = 0u32;
    let mut size = std::mem::size_of::<u32>() as u32;

    unsafe {
        use windows::Win32::System::Registry::RegOpenKeyExW;
        let opened = RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(path_w.as_ptr()),
            0,
            KEY_READ,
            &mut key,
        );
        if opened.is_err() {
            return None;
        }
        let res = RegQueryValueExW(
            key,
            PCWSTR(name_w.as_ptr()),
            None,
            None,
            Some((&mut value as *mut u32).cast()),
            Some(&mut size),
        );
        let _ = RegCloseKey(key);
        if res.is_ok() {
            Some(value)
        } else {
            None
        }
    }
}

/// Temporarily zero the SR frequency throttle so a point can always be
/// created (Windows otherwise silently skips points within 24h), then
/// restore the previous setting.
struct FrequencyGuard {
    previous: Option<u32>,
}

impl FrequencyGuard {
    fn engage() -> Self {
        const FREQ_PATH: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\SystemRestore";
        let path_w: Vec<u16> = FREQ_PATH.encode_utf16().chain(std::iter::once(0)).collect();
        let name_w: Vec<u16> = "SystemRestorePointCreationFrequency"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let previous = read_creation_frequency();

        unsafe {
            use windows::Win32::System::Registry::{RegCreateKeyExW, REG_OPTION_NON_VOLATILE};
            let mut key = HKEY_LOCAL_MACHINE;
            if RegCreateKeyExW(
                HKEY_LOCAL_MACHINE,
                PCWSTR(path_w.as_ptr()),
                0,
                None,
                REG_OPTION_NON_VOLATILE,
                KEY_WRITE | KEY_READ,
                None,
                &mut key,
                None,
            )
            .is_ok()
            {
                let zero = 0u32.to_ne_bytes();
                let _ = RegSetValueExW(key, PCWSTR(name_w.as_ptr()), 0, REG_DWORD, Some(&zero));
                let _ = RegCloseKey(key);
            }
        }

        Self { previous }
    }
}

impl Drop for FrequencyGuard {
    fn drop(&mut self) {
        const FREQ_PATH: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\SystemRestore";
        let path_w: Vec<u16> = FREQ_PATH.encode_utf16().chain(std::iter::once(0)).collect();
        let name_w: Vec<u16> = "SystemRestorePointCreationFrequency"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            use windows::Win32::System::Registry::{
                RegCreateKeyExW, RegDeleteValueW, REG_OPTION_NON_VOLATILE,
            };
            let mut key = HKEY_LOCAL_MACHINE;
            if RegCreateKeyExW(
                HKEY_LOCAL_MACHINE,
                PCWSTR(path_w.as_ptr()),
                0,
                None,
                REG_OPTION_NON_VOLATILE,
                KEY_WRITE | KEY_READ,
                None,
                &mut key,
                None,
            )
            .is_ok()
            {
                match self.previous {
                    Some(mins) => {
                        let bytes = mins.to_ne_bytes();
                        let _ = RegSetValueExW(
                            key,
                            PCWSTR(name_w.as_ptr()),
                            0,
                            REG_DWORD,
                            Some(&bytes),
                        );
                    }
                    None => {
                        let _ = RegDeleteValueW(key, PCWSTR(name_w.as_ptr()));
                    }
                }
                let _ = RegCloseKey(key);
            }
        }
    }
}

/// Create a native Windows System Restore Point ("MODIFY_SETTINGS").
pub fn create_windows_restore_point(description: &str) -> Result<String, String> {
    log_info(
        "restore",
        &format!(
            "Creating native Windows System Restore Point via VSS: '{}'",
            description
        ),
    );

    let Some(sr_set_restore_point) = resolve_srclient() else {
        let msg = "SrClient.dll could not be resolved on this system.".to_string();
        log_warn("restore", &msg);
        return Err(msg);
    };

    // Bypass the 24h creation throttle for the duration of this call.
    let _guard = FrequencyGuard::engage();

    let mut begin_info = RestorePointInfoW {
        dw_event_type: BEGIN_SYSTEM_CHANGE,
        dw_restore_pt_type: MODIFY_SETTINGS,
        ll_sequence_number: 0,
        sz_description: wide_description(&format!("Wino: {}", description)),
    };
    let mut status = StateMgrStatus {
        dw_status: 0,
        ll_sequence_number: 0,
    };

    // SAFETY: both pointers reference valid locals; srclient copies inputs.
    let ok = unsafe { sr_set_restore_point(&mut begin_info, &mut status) };
    if ok == 0 || status.dw_status != ERROR_SUCCESS.0 {
        let code = status.dw_status;
        if code == ERROR_SERVICE_DISABLED || code == ERROR_SERVICE_NOT_ACTIVE {
            let msg = "System Protection is disabled. Enable it via Settings > System > About > System protection to allow automatic restore points.".to_string();
            log_warn("restore", &msg);
            return Err(msg);
        }
        let msg = format!("SRSetRestorePointW failed (Win32 error {}).", code);
        log_warn("restore", &msg);
        return Err(msg);
    }

    let sequence = status.ll_sequence_number;

    // Close the change so the restore point is finalized.
    let mut end_info = RestorePointInfoW {
        dw_event_type: END_SYSTEM_CHANGE,
        dw_restore_pt_type: 0,
        ll_sequence_number: sequence,
        sz_description: wide_description(""),
    };
    let mut end_status = StateMgrStatus {
        dw_status: 0,
        ll_sequence_number: 0,
    };
    let _ = unsafe { sr_set_restore_point(&mut end_info, &mut end_status) };

    let msg = format!(
        "Windows System Restore Point created successfully (sequence {}).",
        sequence
    );
    log_info("restore", &msg);
    Ok(msg)
}
