use crate::monitoring::ram::{get_ram_stats, RamStats};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedMemorySnapshot {
    pub stats: RamStats,
    pub pressure: crate::memory::pressure::MemoryPressure,
    pub compression_enabled: bool,
    pub compressed_bytes: u64,
    pub paged_pool_bytes: u64,
    pub nonpaged_pool_bytes: u64,
}

pub fn capture_memory_snapshot() -> DetailedMemorySnapshot {
    let stats = get_ram_stats();
    let pressure = crate::memory::pressure::calculate_memory_pressure(&stats);
    let (compression_enabled, compressed_bytes) =
        crate::memory::compression::get_compression_stats();
    let paged_pool_bytes = stats.paged_pool_bytes;
    let nonpaged_pool_bytes = stats.nonpaged_pool_bytes;

    DetailedMemorySnapshot {
        stats,
        pressure,
        compression_enabled,
        compressed_bytes,
        paged_pool_bytes,
        nonpaged_pool_bytes,
    }
}
