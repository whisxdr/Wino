use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use windows::Win32::NetworkManagement::IpHelper::{FreeMibTable, GetIfTable2, MIB_IF_ROW2, MIB_IF_TABLE2};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkStats {
    pub bytes_recv_per_sec: u64,
    pub bytes_send_per_sec: u64,
}

pub struct NetworkMonitor {
    last_in_octets: u64,
    last_out_octets: u64,
    last_sample_time: Instant,
    last_stats: NetworkStats,
}

static NET_MONITOR: Mutex<Option<NetworkMonitor>> = Mutex::new(None);

impl Default for NetworkMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkMonitor {
    pub fn new() -> Self {
        let (in_octets, out_octets) = Self::read_total_octets();
        Self {
            last_in_octets: in_octets,
            last_out_octets: out_octets,
            last_sample_time: Instant::now(),
            last_stats: NetworkStats::default(),
        }
    }

    fn read_total_octets() -> (u64, u64) {
        unsafe {
            let mut table_ptr: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
            if GetIfTable2(&mut table_ptr).is_ok() && !table_ptr.is_null() {
                let table = &*table_ptr;
                let num_entries = table.NumEntries as usize;
                let rows_ptr = table.Table.as_ptr() as *const MIB_IF_ROW2;
                let rows = std::slice::from_raw_parts(rows_ptr, num_entries);

                let mut total_in = 0u64;
                let mut total_out = 0u64;

                for row in rows {
                    // Filter out loopback interfaces (type 24 = IF_TYPE_SOFTWARE_LOOPBACK)
                    if row.Type != 24 {
                        total_in += row.InOctets;
                        total_out += row.OutOctets;
                    }
                }

                FreeMibTable(table_ptr as *const _);
                (total_in, total_out)
            } else {
                (0, 0)
            }
        }
    }

    pub fn sample(&mut self) -> NetworkStats {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_sample_time).as_secs_f64();

        if elapsed < 0.1 {
            return self.last_stats.clone();
        }

        let (in_octets, out_octets) = Self::read_total_octets();

        let delta_in = in_octets.saturating_sub(self.last_in_octets);
        let delta_out = out_octets.saturating_sub(self.last_out_octets);

        let recv_speed = (delta_in as f64 / elapsed) as u64;
        let send_speed = (delta_out as f64 / elapsed) as u64;

        self.last_in_octets = in_octets;
        self.last_out_octets = out_octets;
        self.last_sample_time = now;

        self.last_stats = NetworkStats {
            bytes_recv_per_sec: recv_speed,
            bytes_send_per_sec: send_speed,
        };

        self.last_stats.clone()
    }
}

pub fn get_network_stats() -> NetworkStats {
    let mut lock = NET_MONITOR.lock();
    if lock.is_none() {
        *lock = Some(NetworkMonitor::new());
    }
    if let Some(monitor) = lock.as_mut() {
        monitor.sample()
    } else {
        NetworkStats::default()
    }
}
