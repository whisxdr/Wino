//! Power-friendly automatic memory trimmer.
//!
//! A single long-lived background thread periodically samples total RAM usage
//! and triggers a safe working-set trim when usage exceeds a configurable
//! threshold. A cooldown prevents thrashing; the whole loop can be toggled or
//! stopped at runtime (used by Settings and app shutdown).

use crate::memory::monitor::capture_memory_snapshot;
use crate::memory::optimizer::optimize_memory;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Pure decision logic (unit-testable): should we trim now?
pub fn should_trim(usage_pct: f32, threshold_pct: f32, since_last_trim: Duration, cooldown: Duration) -> bool {
    if usage_pct < threshold_pct {
        return false;
    }
    since_last_trim >= cooldown
}

pub struct AutoTrimHandle {
    pub enabled: Arc<AtomicBool>,
    /// Whole-percent threshold at which the trimmer kicks in.
    pub threshold_pct: Arc<AtomicU32>,
    stop: Arc<AtomicBool>,
}

impl AutoTrimHandle {
    pub fn set_enabled(&self, on: bool) {
        self.enabled.store(on, Ordering::Release);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }
}

impl Drop for AutoTrimHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}

impl AutoTrimHandle {
    /// Spawn the monitor thread. `result_tx` receives log events surfaced in
    /// the UI event ticker / audit log.
    pub fn spawn(result_tx: Sender<crate::app::worker::JobResult>, enabled: bool) -> Option<Self> {
        let flag_enabled = Arc::new(AtomicBool::new(enabled));
        let threshold = Arc::new(AtomicU32::new(85));
        let stop = Arc::new(AtomicBool::new(false));

        let en = Arc::clone(&flag_enabled);
        let th = Arc::clone(&threshold);
        let st = Arc::clone(&stop);

        let spawned = std::thread::Builder::new()
            .name("wino-auto-trim".to_string())
            .spawn(move || {
                let _interval = Duration::from_secs(30);
                let cooldown = Duration::from_secs(300);
                let mut last_trim = Instant::now() - cooldown;

                loop {
                    // Sleep in short slices so shutdown stays responsive.
                    for _ in 0..30 {
                        if st.load(Ordering::Acquire) {
                            return;
                        }
                        std::thread::sleep(Duration::from_secs(1));
                    }

                    if !en.load(Ordering::Acquire) {
                        continue;
                    }

                    let snapshot = capture_memory_snapshot();
                    let usage = snapshot.stats.usage_pct;
                    let limit = th.load(Ordering::Acquire) as f32;

                    if !should_trim(usage, limit, last_trim.elapsed(), cooldown) {
                        continue;
                    }

                    last_trim = Instant::now();
                    let _ = result_tx.send(crate::app::worker::JobResult::Event {
                        category: "Auto-Trim".to_string(),
                        message: format!("RAM usage {:.1}% exceeded {}% threshold — starting safe working-set trim.", usage, limit),
                        is_success: true,
                    });

                    let report = optimize_memory(false);
                    let _ = result_tx.send(crate::app::worker::JobResult::Event {
                        category: "Auto-Trim".to_string(),
                        message: report.message,
                        is_success: true,
                    });
                }
            })
            .ok();

        spawned.as_ref()?;

        Some(Self { enabled: flag_enabled, threshold_pct: threshold, stop })
    }
}

