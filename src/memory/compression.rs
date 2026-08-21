use crate::core::executor::SystemExecutor;

pub fn get_compression_stats() -> (bool, u64) {
    // Check if Memory Compression (MMAgent) is enabled via registry
    // HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Memory Management
    let enabled_val = SystemExecutor::read_registry_dword(
        "HKLM",
        "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Memory Management",
        "MemoryCompression",
    );

    let is_enabled = enabled_val.unwrap_or(1) != 0;

    // Approximate compressed memory from System process or cache
    let ram_stats = crate::monitoring::ram::get_ram_stats();
    let estimated_compressed = if is_enabled {
        // Typically around 5-15% of used memory on modern Windows
        (ram_stats.used_bytes as f64 * 0.08) as u64
    } else {
        0
    };

    (is_enabled, estimated_compressed)
}
