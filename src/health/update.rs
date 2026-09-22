use serde::{Deserialize, Serialize};
use windows::core::PCWSTR;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ,
};

/// Health of the Windows Update subsystem.
///
/// `service_state` is the *observed* state of `wuauserv`. Earlier versions
/// reported a hard-coded `true` here, which made the health dashboard claim
/// Windows Update was fine even when the service was stopped or disabled. It is
/// now a real query, and `None` means "could not be determined".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateHealth {
    /// Observed `wuauserv` state, or `None` when it could not be queried.
    pub service_state: Option<String>,
    pub pending_reboot: bool,
}

impl UpdateHealth {
    /// Whether the Windows Update service was observed running.
    pub fn service_running(&self) -> bool {
        matches!(self.service_state.as_deref(), Some("Running"))
    }

    /// Whether the service was observed in a non-running, queryable state.
    pub fn service_stopped(&self) -> bool {
        matches!(self.service_state.as_deref(), Some("Stopped"))
    }
}

pub fn check_update_health() -> UpdateHealth {
    UpdateHealth {
        service_state: crate::services::scanner::service_state_query("wuauserv"),
        pending_reboot: check_reboot_required(),
    }
}

fn check_reboot_required() -> bool {
    // Any one of these keys present means a restart is pending.
    const REBOOT_KEYS: &[&str] = &[
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\WindowsUpdate\\Auto Update\\RebootRequired",
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Component Based Servicing\\RebootPending",
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Component Based Servicing\\RebootInProgress",
    ];

    REBOOT_KEYS.iter().any(|key| registry_key_exists(key))
}

fn registry_key_exists(path: &str) -> bool {
    let path_wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    let mut hkey = HKEY::default();

    unsafe {
        if RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(path_wide.as_ptr()),
            0,
            KEY_READ,
            &mut hkey,
        ) == ERROR_SUCCESS
        {
            let _ = RegCloseKey(hkey);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_running_requires_an_observed_running_state() {
        let running = UpdateHealth {
            service_state: Some("Running".to_string()),
            pending_reboot: false,
        };
        assert!(running.service_running());
        assert!(!running.service_stopped());

        let stopped = UpdateHealth {
            service_state: Some("Stopped".to_string()),
            pending_reboot: false,
        };
        assert!(!stopped.service_running());
        assert!(stopped.service_stopped());
    }

    #[test]
    fn unknown_service_state_is_not_treated_as_running() {
        let unknown = UpdateHealth {
            service_state: None,
            pending_reboot: false,
        };
        assert!(!unknown.service_running());
        assert!(!unknown.service_stopped());
    }
}
