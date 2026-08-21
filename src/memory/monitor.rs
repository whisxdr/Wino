use crate::monitoring::ram::{get_ram_stats, RamStats};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedMemorySnapshot {
    pub stats: RamStats,
    pub pressure: crate::memory::pressure::MemoryPressure,
    pub compression_enabled: bool,
    pub compressed_bytes: u64,
}

pub fn capture_memory_snapshot() -> DetailedMemorySnapshot {
    let stats = get_ram_stats();
    let pressure = crate::memory::pressure::calculate_memory_pressure(&stats);
    let (compression_enabled, compressed_bytes) = crate::memory::compression::get_compression_stats();

    DetailedMemorySnapshot {
        stats,
        pressure,
        compression_enabled,
        compressed_bytes,
    }
}
