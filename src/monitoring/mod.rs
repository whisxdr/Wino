pub mod cpu;
pub mod disk;
pub mod gpu;
pub mod network;
pub mod process;
pub mod ram;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemMetricsSnapshot {
    pub cpu_usage_pct: f32,
    pub ram_total_bytes: u64,
    pub ram_used_bytes: u64,
    pub ram_available_bytes: u64,
    pub ram_cached_bytes: u64,
    pub ram_usage_pct: f32,
    pub commit_used_bytes: u64,
    pub commit_limit_bytes: u64,
    pub disk_total_bytes: u64,
    pub disk_free_bytes: u64,
    pub disk_usage_pct: f32,
    pub net_recv_bytes_per_sec: u64,
    pub net_send_bytes_per_sec: u64,
    pub gpu_name: String,
    pub gpu_vram_total_bytes: u64,
    pub gpu_vram_used_bytes: u64,
    pub process_count: usize,
}
