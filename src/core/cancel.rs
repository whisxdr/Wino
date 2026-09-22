//! Cooperative cancellation for long-running scans.
//!
//! Storage analysis and application enumeration walk trees whose size the user
//! controls. A scan checks its token at every directory boundary and bails out
//! early instead of pinning a worker thread for minutes. The token is a plain
//! atomic flag, so signalling it from the UI thread costs nothing.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Shared cancellation flag handed to a scan.
#[derive(Debug, Clone, Default)]
pub struct CancelToken {
    flag: Arc<AtomicBool>,
}

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ask the running scan to stop at its next checkpoint.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::Release);
    }

    /// True once [`Self::cancel`] has been called.
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Acquire)
    }

    /// Clear the flag so the token can be reused for a fresh scan.
    pub fn reset(&self) {
        self.flag.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_starts_clear_and_trips_once_cancelled() {
        let token = CancelToken::new();
        assert!(!token.is_cancelled());

        token.cancel();
        assert!(token.is_cancelled());

        token.reset();
        assert!(!token.is_cancelled());
    }

    #[test]
    fn clones_share_one_flag() {
        let token = CancelToken::new();
        let observer = token.clone();
        token.cancel();
        assert!(observer.is_cancelled());
    }
}
