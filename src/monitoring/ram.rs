use serde::{Deserialize, Serialize};
use windows::Win32::System::ProcessStatus::{GetPerformanceInfo, PERFORMANCE_INFORMATION};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RamStats {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub cached_bytes: u64,
    pub paged_pool_bytes: u64,
    pub nonpaged_pool_bytes: u64,
    pub commit_used_bytes: u64,
    pub commit_limit_bytes: u64,
    pub usage_pct: f32,
    pub process_count: usize,
    pub handle_count: u32,
    pub thread_count: u32,
}

pub fn get_ram_stats() -> RamStats {
    unsafe {
        let mut mem_status = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };

        let mut perf_info = PERFORMANCE_INFORMATION {
            cb: std::mem::size_of::<PERFORMANCE_INFORMATION>() as u32,
            ..Default::default()
        };

        let mem_ok = GlobalMemoryStatusEx(&mut mem_status).is_ok();
        let perf_ok = GetPerformanceInfo(
            &mut perf_info,
            std::mem::size_of::<PERFORMANCE_INFORMATION>() as u32,
        )
        .is_ok();

        if !mem_ok {
            return RamStats::default();
        }

        let total_bytes = mem_status.ullTotalPhys;
        let available_bytes = mem_status.ullAvailPhys;
        let used_bytes = total_bytes.saturating_sub(available_bytes);
        let usage_pct = if total_bytes > 0 {
            (used_bytes as f32 / total_bytes as f32) * 100.0
        } else {
            0.0
        };

        let page_size = if perf_ok && perf_info.PageSize > 0 {
            perf_info.PageSize as u64
        } else {
            4096
        };

        let cached_bytes = if perf_ok {
            (perf_info.SystemCache as u64) * page_size
        } else {
            0
        };

        let paged_pool_bytes = if perf_ok {
            (perf_info.KernelPaged as u64) * page_size
        } else {
            0
        };

        let nonpaged_pool_bytes = if perf_ok {
            (perf_info.KernelNonpaged as u64) * page_size
        } else {
            0
        };

        let commit_used_bytes = if perf_ok {
            (perf_info.CommitTotal as u64) * page_size
        } else {
            mem_status
                .ullTotalPageFile
                .saturating_sub(mem_status.ullAvailPageFile)
        };

        let commit_limit_bytes = if perf_ok {
            (perf_info.CommitLimit as u64) * page_size
        } else {
            mem_status.ullTotalPageFile
        };

        let (process_count, handle_count, thread_count) = if perf_ok {
            (
                perf_info.ProcessCount as usize,
                perf_info.HandleCount,
                perf_info.ThreadCount,
            )
        } else {
            (0, 0, 0)
        };

        RamStats {
            total_bytes,
            used_bytes,
            available_bytes,
            cached_bytes,
            paged_pool_bytes,
            nonpaged_pool_bytes,
            commit_used_bytes,
            commit_limit_bytes,
            usage_pct,
            process_count,
            handle_count,
            thread_count,
        }
    }
}

pub fn get_total_ram_bytes() -> u64 {
    unsafe {
        let mut mem_status = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };
        if GlobalMemoryStatusEx(&mut mem_status).is_ok() {
            mem_status.ullTotalPhys
        } else {
            0
        }
    }
}
