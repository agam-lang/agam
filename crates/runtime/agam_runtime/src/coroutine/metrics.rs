//! Asynchronous Runtime Metrics & Diagnostics.
//!
//! Provides execution introspection, latency metrics, and scheduler statistics.

use std::sync::atomic::{AtomicU64, Ordering};

/// Snapshot of runtime execution statistics.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuntimeMetrics {
    pub tasks_spawned: u64,
    pub tasks_completed: u64,
    pub tasks_cancelled: u64,
    pub steals_attempted: u64,
    pub steals_successful: u64,
}

pub(crate) struct AtomicRuntimeMetrics {
    pub tasks_spawned: AtomicU64,
    pub tasks_completed: AtomicU64,
    pub tasks_cancelled: AtomicU64,
    pub steals_attempted: AtomicU64,
    pub steals_successful: AtomicU64,
}

impl AtomicRuntimeMetrics {
    pub fn new() -> Self {
        Self {
            tasks_spawned: AtomicU64::new(0),
            tasks_completed: AtomicU64::new(0),
            tasks_cancelled: AtomicU64::new(0),
            steals_attempted: AtomicU64::new(0),
            steals_successful: AtomicU64::new(0),
        }
    }

    pub fn snapshot(&self) -> RuntimeMetrics {
        RuntimeMetrics {
            tasks_spawned: self.tasks_spawned.load(Ordering::Relaxed),
            tasks_completed: self.tasks_completed.load(Ordering::Relaxed),
            tasks_cancelled: self.tasks_cancelled.load(Ordering::Relaxed),
            steals_attempted: self.steals_attempted.load(Ordering::Relaxed),
            steals_successful: self.steals_successful.load(Ordering::Relaxed),
        }
    }
}
