use crate::core::logger::{log_error, log_info, log_warn};
use crate::restore::snapshots::create_snapshot;
use windows::core::PCWSTR;
use windows::Win32::System::Services::{
    ChangeServiceConfigW, CloseServiceHandle, OpenSCManagerW, OpenServiceW, SC_MANAGER_ALL_ACCESS,
    SERVICE_CHANGE_CONFIG, SERVICE_NO_CHANGE, SERVICE_START_TYPE,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupType {
    Automatic = 2,
    Manual = 3,
    Disabled = 4,
}

pub fn set_service_startup(service_name: &str, startup: StartupType) -> Result<(), String> {
    // Safety check
    let lower = service_name.to_lowercase();
    if matches!(lower.as_str(), "rpcss" | "dcomlaunch" | "eventlog" | "bfe" | "dhcp" | "dnscache" | "windefend" | "samss" | "lsass") {
        log_warn("services", &format!("BLOCKED: Attempted to modify critical system service: {}", service_name));
        return Err(format!("Modification of critical system service '{}' is prohibited for stability.", service_name));
    }

    let _ = create_snapshot(&format!("Service startup change: {}", service_name));

    let svc_name_wide: Vec<u16> = service_name.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let scm_res = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_ALL_ACCESS);
        let Ok(scm) = scm_res else {
            return Err("Failed to open Service Control Manager (Administrator rights required).".to_string());
        };

        let svc_res = OpenServiceW(
            scm,
            PCWSTR(svc_name_wide.as_ptr()),
            SERVICE_CHANGE_CONFIG,
        );

        let Ok(svc) = svc_res else {
            let _ = CloseServiceHandle(scm);
            return Err(format!("Failed to open service '{}'. Access denied or service not found.", service_name));
        };

        let res = ChangeServiceConfigW(
            svc,
            windows::Win32::System::Services::ENUM_SERVICE_TYPE(SERVICE_NO_CHANGE),
            SERVICE_START_TYPE(startup as u32),
            windows::Win32::System::Services::SERVICE_ERROR(SERVICE_NO_CHANGE),
            PCWSTR::null(),
            PCWSTR::null(),
            None,
            PCWSTR::null(),
            PCWSTR::null(),
            PCWSTR::null(),
            PCWSTR::null(),
        );

        let _ = CloseServiceHandle(svc);
        let _ = CloseServiceHandle(scm);

        if res.is_ok() {
            log_info("services", &format!("Changed startup type for {} to {:?}", service_name, startup));
            Ok(())
        } else {
            log_error("services", &format!("Failed to change startup type for {}", service_name));
            Err(format!("Failed to update service config for {}", service_name))
        }
    }
}
