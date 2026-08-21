use serde::{Deserialize, Serialize};
use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GpuStats {
    pub name: String,
    pub dedicated_vram_bytes: u64,
    pub shared_system_memory_bytes: u64,
}

pub fn get_gpu_stats() -> GpuStats {
    unsafe {
        let factory_res: windows::core::Result<IDXGIFactory1> = CreateDXGIFactory1();
        if let Ok(factory) = factory_res {
            let mut adapter_index = 0u32;
            while let Ok(adapter) = factory.EnumAdapters1(adapter_index) {
                if let Ok(desc) = adapter.GetDesc1() {
                    // Skip Microsoft Basic Render Driver / Software adapter
                    let is_software = (desc.Flags & 2) != 0; // DXGI_ADAPTER_FLAG_SOFTWARE
                    let name = String::from_utf16_lossy(&desc.Description)
                        .trim_matches('\0')
                        .trim()
                        .to_string();

                    if !is_software && !name.is_empty() {
                        return GpuStats {
                            name,
                            dedicated_vram_bytes: desc.DedicatedVideoMemory as u64,
                            shared_system_memory_bytes: desc.SharedSystemMemory as u64,
                        };
                    }
                }
                adapter_index += 1;
            }
        }
    }

    GpuStats {
        name: "Integrated / Standard Graphics".to_string(),
        dedicated_vram_bytes: 0,
        shared_system_memory_bytes: 0,
    }
}
