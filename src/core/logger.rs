use chrono::Local;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

#[derive(Clone)]
pub struct MemoryLogger {
    entries: Arc<Mutex<VecDeque<LogEntry>>>,
    max_entries: usize,
}

static GLOBAL_LOGGER: parking_lot::RwLock<Option<MemoryLogger>> = parking_lot::RwLock::new(None);

impl MemoryLogger {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(max_entries))),
            max_entries,
        }
    }

    pub fn init_global(max_entries: usize) -> Self {
        let logger = Self::new(max_entries);
        *GLOBAL_LOGGER.write() = Some(logger.clone());
        logger
    }

    pub fn global() -> Option<Self> {
        GLOBAL_LOGGER.read().clone()
    }

    pub fn log(&self, level: &str, target: &str, message: &str) {
        let entry = LogEntry {
            timestamp: Local::now().format("%H:%M:%S").to_string(),
            level: level.to_string(),
            target: target.to_string(),
            message: message.to_string(),
        };

        let mut lock = self.entries.lock();
        if lock.len() >= self.max_entries {
            lock.pop_front();
        }
        lock.push_back(entry);
    }

    pub fn get_entries(&self) -> Vec<LogEntry> {
        self.entries.lock().iter().cloned().collect()
    }

    pub fn clear(&self) {
        self.entries.lock().clear();
    }
}

pub fn log_info(target: &str, message: &str) {
    tracing::info!("[{}] {}", target, message);
    if let Some(logger) = MemoryLogger::global() {
        logger.log("INFO", target, message);
    }
}

pub fn log_warn(target: &str, message: &str) {
    tracing::warn!("[{}] {}", target, message);
    if let Some(logger) = MemoryLogger::global() {
        logger.log("WARN", target, message);
    }
}

pub fn log_error(target: &str, message: &str) {
    tracing::error!("[{}] {}", target, message);
    if let Some(logger) = MemoryLogger::global() {
        logger.log("ERROR", target, message);
    }
}
