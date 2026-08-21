use serde::{Deserialize, Serialize};
use windows::core::PCWSTR;
use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiskStats {
    pub drive_letter: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub used_bytes: u64,
    pub usage_pct: f32,
}

pub fn get_primary_disk_stats() -> DiskStats {
    let drive_path = "C:\\\0";
    let drive_wide: Vec<u16> = drive_path.encode_utf16().collect();

    unsafe {
        let mut free_bytes_available = 0u64;
        let mut total_bytes = 0u64;
        let mut total_free_bytes = 0u64;

        if GetDiskFreeSpaceExW(
            PCWSTR(drive_wide.as_ptr()),
            Some(&mut free_bytes_available),
            Some(&mut total_bytes),
            Some(&mut total_free_bytes),
        ).is_ok() && total_bytes > 0 {
            let used_bytes = total_bytes.saturating_sub(total_free_bytes);
            let usage_pct = (used_bytes as f32 / total_bytes as f32) * 100.0;

            DiskStats {
                drive_letter: "C:".to_string(),
                total_bytes,
                free_bytes: total_free_bytes,
                used_bytes,
                usage_pct,
            }
        } else {
            DiskStats::default()
        }
    }
}
