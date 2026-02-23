use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct SyncState {
    last_sync: Mutex<Option<Instant>>,
    last_full_sync: Mutex<Option<Instant>>,
    sync_interval: Duration,
    manual_trigger: AtomicBool,
    manual_full_trigger: AtomicBool,
    sync_in_progress: AtomicBool,
}

impl SyncState {
    pub fn new(sync_interval: Duration) -> Self {
        Self {
            last_sync: Mutex::new(None),
            last_full_sync: Mutex::new(None),
            sync_interval,
            manual_trigger: AtomicBool::new(false),
            manual_full_trigger: AtomicBool::new(false),
            sync_in_progress: AtomicBool::new(false),
        }
    }

    pub fn mark_sync_complete(&self) {
        let mut guard = self.last_sync.lock().expect("last_sync mutex poisoned");
        *guard = Some(Instant::now());
    }

    pub fn last_sync(&self) -> Option<Instant> {
        *self.last_sync.lock().expect("last_sync mutex poisoned")
    }

    pub fn mark_full_sync_complete(&self) {
        let mut guard = self
            .last_full_sync
            .lock()
            .expect("last_full_sync mutex poisoned");
        *guard = Some(Instant::now());
    }

    pub fn last_full_sync(&self) -> Option<Instant> {
        *self
            .last_full_sync
            .lock()
            .expect("last_full_sync mutex poisoned")
    }

    pub fn seconds_until_next_sync(&self) -> u64 {
        let guard = self.last_sync.lock().expect("last_sync mutex poisoned");
        match *guard {
            Some(last) => {
                let elapsed = last.elapsed();
                if elapsed >= self.sync_interval {
                    0
                } else {
                    (self.sync_interval - elapsed).as_secs()
                }
            }
            None => 0,
        }
    }

    pub fn trigger_manual(&self) {
        self.manual_trigger.store(true, Ordering::Relaxed);
    }

    pub fn check_and_clear_manual_trigger(&self) -> bool {
        self.manual_trigger.swap(false, Ordering::Relaxed)
    }

    pub fn trigger_manual_full(&self) {
        self.manual_full_trigger.store(true, Ordering::Relaxed);
    }

    pub fn check_and_clear_manual_full_trigger(&self) -> bool {
        self.manual_full_trigger.swap(false, Ordering::Relaxed)
    }

    pub fn sync_interval(&self) -> Duration {
        self.sync_interval
    }

    pub fn mark_sync_start(&self) -> bool {
        self.sync_in_progress
            .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
    }

    pub fn mark_sync_end(&self) {
        self.sync_in_progress.store(false, Ordering::Relaxed);
    }

    pub fn is_sync_in_progress(&self) -> bool {
        self.sync_in_progress.load(Ordering::Relaxed)
    }
}
