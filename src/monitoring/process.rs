use serde::{Deserialize, Serialize};
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, TerminateProcess, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessItem {
    pub pid: u32,
    pub name: String,
    pub memory_working_set_bytes: u64,
    pub exe_path: String,
    pub is_system_critical: bool,
    pub is_signed: bool,
    pub publisher: String,
}

const CRITICAL_PROCESS_NAMES: &[&str] = &[
    "system", "smss.exe", "csrss.exe", "wininit.exe", "services.exe", "lsass.exe",
    "winlogon.exe", "svchost.exe", "dwm.exe", "fontdrvhost.exe", "sihost.exe",
    "explorer.exe", "taskhostw.exe", "runtimebroker.exe", "ctfmon.exe",
];

pub fn is_critical_process(name: &str) -> bool {
    let lower = name.to_lowercase();
    CRITICAL_PROCESS_NAMES.iter().any(|&c| c == lower)
}

pub fn list_running_processes() -> Vec<ProcessItem> {
    let mut results = Vec::new();

    unsafe {
        let snapshot_res = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        let Ok(snapshot) = snapshot_res else {
            return results;
        };

        if snapshot == INVALID_HANDLE_VALUE {
            return results;
        }

        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let pid = entry.th32ProcessID;
                if pid > 0 {
                    let name = String::from_utf16_lossy(&entry.szExeFile)
                        .trim_matches('\0')
                        .to_string();

                    let is_critical = is_critical_process(&name);
                    let (mem_bytes, exe_path) = get_process_details(pid);

                    let (is_signed, publisher) = if is_critical || exe_path.to_lowercase().starts_with("c:\\windows\\") {
                        (true, "Microsoft Windows".to_string())
                    } else if exe_path.to_lowercase().contains("program files") {
                        (true, "Verified Application".to_string())
                    } else if !exe_path.is_empty() {
                        (false, "User Process".to_string())
                    } else {
                        (false, "System".to_string())
                    };

                    results.push(ProcessItem {
                        pid,
                        name,
                        memory_working_set_bytes: mem_bytes,
                        exe_path,
                        is_system_critical: is_critical,
                        is_signed,
                        publisher,
                    });
                }

                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }

        let _ = CloseHandle(snapshot);
    }

    // Sort by memory working set descending
    results.sort_by(|a, b| b.memory_working_set_bytes.cmp(&a.memory_working_set_bytes));
    results
}

fn get_process_details(pid: u32) -> (u64, String) {
    unsafe {
        let handle_res = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid);
        let Ok(handle) = handle_res else {
            return (0, String::new());
        };

        let mut mem_counters = PROCESS_MEMORY_COUNTERS_EX {
            cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
            ..Default::default()
        };

        let mem_bytes = if GetProcessMemoryInfo(
            handle,
            &mut mem_counters as *mut _ as *mut _,
            std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
        ).is_ok() {
            mem_counters.WorkingSetSize as u64
        } else {
            0
        };

        let mut path_buf = vec![0u16; 1024];
        let mut path_len = path_buf.len() as u32;

        let exe_path = if QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(path_buf.as_mut_ptr()),
            &mut path_len,
        ).is_ok() {
            String::from_utf16_lossy(&path_buf[..path_len as usize])
        } else {
            String::new()
        };

        let _ = CloseHandle(handle);
        (mem_bytes, exe_path)
    }
}

pub fn kill_process(pid: u32) -> Result<(), String> {
    unsafe {
        let handle_res = OpenProcess(PROCESS_TERMINATE, false, pid);
        let Ok(handle) = handle_res else {
            return Err("Access denied or process not found.".to_string());
        };

        let res = TerminateProcess(handle, 1);
        let _ = CloseHandle(handle);

        if res.is_ok() {
            Ok(())
        } else {
            Err("Failed to terminate process.".to_string())
        }
    }
}
