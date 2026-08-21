use serde::{Deserialize, Serialize};
use windows::core::PCWSTR;
use windows::Win32::System::Services::{
    CloseServiceHandle, EnumServicesStatusExW, OpenSCManagerW, ENUM_SERVICE_STATUS_PROCESSW, SC_ENUM_PROCESS_INFO, SC_MANAGER_ENUMERATE_SERVICE, SERVICE_STATE_ALL, SERVICE_WIN32,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceItem {
    pub service_name: String,
    pub display_name: String,
    pub status: String,       // "Running", "Stopped", "Paused"
    pub classification: String, // "Safe to change", "Usually safe", "Optional", "Do not touch"
    pub recommended_startup: String,
    pub risk: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRule {
    pub service_name: String,
    pub display_name: String,
    pub description: String,
    pub classification: String,
    pub recommended_startup: String,
    pub risk: String,
    pub reason: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

pub fn load_service_rules() -> Vec<ServiceRule> {
    const RULES_JSON: &str = include_str!("../../data/service_rules.json");
    serde_json::from_str(RULES_JSON).unwrap_or_default()
}

pub fn scan_services() -> Vec<ServiceItem> {
    let rules = load_service_rules();
    let mut items = Vec::new();

    unsafe {
        let scm_res = OpenSCManagerW(
            PCWSTR::null(),
            PCWSTR::null(),
            SC_MANAGER_ENUMERATE_SERVICE,
        );

        let Ok(scm) = scm_res else {
            return fallback_rules_scan(&rules);
        };

        let mut bytes_needed = 0u32;
        let mut services_returned = 0u32;
        let mut resume_handle = 0u32;

        let _ = EnumServicesStatusExW(
            scm,
            SC_ENUM_PROCESS_INFO,
            SERVICE_WIN32,
            SERVICE_STATE_ALL,
            None,
            &mut bytes_needed,
            &mut services_returned,
            Some(&mut resume_handle),
            PCWSTR::null(),
        );

        if bytes_needed > 0 {
            let mut buffer = vec![0u8; bytes_needed as usize];
            let success = EnumServicesStatusExW(
                scm,
                SC_ENUM_PROCESS_INFO,
                SERVICE_WIN32,
                SERVICE_STATE_ALL,
                Some(&mut buffer),
                &mut bytes_needed,
                &mut services_returned,
                Some(&mut resume_handle),
                PCWSTR::null(),
            );

            if success.is_ok() && services_returned > 0 {
                let services_slice = std::slice::from_raw_parts(
                    buffer.as_ptr() as *const ENUM_SERVICE_STATUS_PROCESSW,
                    services_returned as usize,
                );

                for svc in services_slice {
                    let s_name = String::from_utf16_lossy(svc.lpServiceName.as_wide()).trim().to_string();
                    let d_name = String::from_utf16_lossy(svc.lpDisplayName.as_wide()).trim().to_string();

                    let status_str = match svc.ServiceStatusProcess.dwCurrentState.0 {
                        4 => "Running",
                        1 => "Stopped",
                        7 => "Paused",
                        2 => "Starting",
                        3 => "Stopping",
                        _ => "Unknown",
                    };

                    let matched_rule = rules.iter().find(|r| r.service_name.eq_ignore_ascii_case(&s_name));

                    let (classification, rec_startup, risk, reason) = if let Some(r) = matched_rule {
                        (r.classification.clone(), r.recommended_startup.clone(), r.risk.clone(), r.reason.clone())
                    } else if is_essential_system_service(&s_name) {
                        ("Do not touch".to_string(), "Automatic".to_string(), "Critical".to_string(), "Core Windows OS dependency.".to_string())
                    } else {
                        ("Optional".to_string(), "Default".to_string(), "Low".to_string(), "Standard Windows service.".to_string())
                    };

                    items.push(ServiceItem {
                        service_name: s_name,
                        display_name: d_name,
                        status: status_str.to_string(),
                        classification,
                        recommended_startup: rec_startup,
                        risk,
                        reason,
                    });
                }
            }
        }

        let _ = CloseServiceHandle(scm);
    }

    // Sort items: "Safe to change" first, then "Optional", then "Do not touch"
    items.sort_by(|a, b| {
        let rank = |c: &str| match c {
            "Safe to change" => 1,
            "Usually safe" => 2,
            "Optional" => 3,
            _ => 4,
        };
        rank(&a.classification).cmp(&rank(&b.classification))
    });

    items
}

fn fallback_rules_scan(rules: &[ServiceRule]) -> Vec<ServiceItem> {
    rules.iter().map(|r| ServiceItem {
        service_name: r.service_name.clone(),
        display_name: r.display_name.clone(),
        status: "Unknown".to_string(),
        classification: r.classification.clone(),
        recommended_startup: r.recommended_startup.clone(),
        risk: r.risk.clone(),
        reason: r.reason.clone(),
    }).collect()
}

fn is_essential_system_service(name: &str) -> bool {
    let lower = name.to_lowercase();
    matches!(lower.as_str(), "rpcss" | "dcomlaunch" | "eventlog" | "bfe" | "dhcp" | "dnscache" | "windefend" | "wuauserv" | "samss" | "lsass")
}
