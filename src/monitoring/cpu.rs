use parking_lot::Mutex;
use std::time::Instant;
use windows::Win32::Foundation::FILETIME;
use windows::Win32::System::Threading::GetSystemTimes;

pub struct CpuMonitor {
    last_idle: u64,
    last_kernel: u64,
    last_user: u64,
    last_sample_time: Instant,
    last_cpu_pct: f32,
}

static CPU_MONITOR: Mutex<Option<CpuMonitor>> = Mutex::new(None);

impl CpuMonitor {
    pub fn new() -> Self {
        let (idle, kernel, user) = Self::read_raw_times();
        Self {
            last_idle: idle,
            last_kernel: kernel,
            last_user: user,
            last_sample_time: Instant::now(),
            last_cpu_pct: 0.0,
        }
    }

    fn filetime_to_u64(ft: FILETIME) -> u64 {
        ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64)
    }

    fn read_raw_times() -> (u64, u64, u64) {
        unsafe {
            let mut idle = FILETIME::default();
            let mut kernel = FILETIME::default();
            let mut user = FILETIME::default();

            if GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)).is_ok() {
                (
                    Self::filetime_to_u64(idle),
                    Self::filetime_to_u64(kernel),
                    Self::filetime_to_u64(user),
                )
            } else {
                (0, 0, 0)
            }
        }
    }

    pub fn sample(&mut self) -> f32 {
        let (idle, kernel, user) = Self::read_raw_times();

        let delta_idle = idle.saturating_sub(self.last_idle);
        let delta_kernel = kernel.saturating_sub(self.last_kernel);
        let delta_user = user.saturating_sub(self.last_user);

        let delta_total = delta_kernel + delta_user;

        self.last_idle = idle;
        self.last_kernel = kernel;
        self.last_user = user;
        self.last_sample_time = Instant::now();

        if delta_total == 0 {
            return self.last_cpu_pct;
        }

        // On Windows, kernel time includes idle time
        let busy_time = delta_total.saturating_sub(delta_idle);
        let pct = (busy_time as f32 / delta_total as f32) * 100.0;
        let clamped = pct.clamp(0.0, 100.0);
        self.last_cpu_pct = clamped;
        clamped
    }
}

pub fn get_cpu_usage() -> f32 {
    let mut lock = CPU_MONITOR.lock();
    if lock.is_none() {
        *lock = Some(CpuMonitor::new());
    }
    if let Some(monitor) = lock.as_mut() {
        monitor.sample()
    } else {
        0.0
    }
}
