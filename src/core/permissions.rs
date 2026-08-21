use std::ptr;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND};
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;

/// Checks whether the current process is running with elevated Administrator privileges.
pub fn is_admin() -> bool {
    unsafe {
        let mut token: HANDLE = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }

        let mut elevation = TOKEN_ELEVATION::default();
        let mut ret_len = 0u32;
        let success = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret_len,
        );

        let _ = CloseHandle(token);

        if success.is_ok() {
            elevation.TokenIsElevated != 0
        } else {
            false
        }
    }
}

/// Request UAC elevation by restarting the current executable with "runas"
pub fn restart_elevated() -> Result<(), String> {
    if is_admin() {
        return Ok(());
    }

    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_wide: Vec<u16> = current_exe.to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect();
    let runas_wide: Vec<u16> = "runas\0".encode_utf16().collect();

    unsafe {
        let instance = ShellExecuteW(
            HWND(ptr::null_mut()),
            PCWSTR(runas_wide.as_ptr()),
            PCWSTR(exe_wide.as_ptr()),
            PCWSTR(ptr::null()),
            PCWSTR(ptr::null()),
            SW_SHOW,
        );

        if instance.0 as usize <= 32 {
            return Err("Failed to elevate process via UAC prompt.".to_string());
        }
    }

    std::process::exit(0);
}
