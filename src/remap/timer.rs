//! Cancellable one-shot timer used by the keyboard hook to fire a Timeout
//! event into the FSM after the configured tap threshold.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub struct TimerHandle {
    cancelled: Arc<AtomicBool>,
    _thread: Option<thread::JoinHandle<()>>,
}

impl TimerHandle {
    pub fn start<F: FnOnce() + Send + 'static>(delay_ms: u64, callback: F) -> Self {
        let cancelled = Arc::new(AtomicBool::new(false));
        let flag = cancelled.clone();
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(delay_ms));
            if !flag.load(Ordering::SeqCst) {
                callback();
            }
        });
        Self {
            cancelled,
            _thread: Some(handle),
        }
    }

    pub fn cancel(self) {
        self.cancelled.store(true, Ordering::SeqCst);
        // Let the thread finish naturally; cancellation just sets the flag.
    }
}
