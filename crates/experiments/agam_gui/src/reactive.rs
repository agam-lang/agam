//! # Reactive State Signals & Frame Batch Scheduler (`agam_gui::reactive`)
//!
//! Provides fine-grained reactive state tracking, dependency recording,
//! and dirty subtree reconciliation scheduled to vsync frame boundaries.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

static NEXT_SIGNAL_ID: AtomicU64 = AtomicU64::new(1);

/// Unique monotonic identifier for a reactive state signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignalId(pub u64);

impl SignalId {
    pub fn next() -> Self {
        Self(NEXT_SIGNAL_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// Listener callback notified when signal value mutates.
pub type SignalListener = Arc<dyn Fn() + Send + Sync>;

/// Fine-grained reactive signal holding a value of type `T`.
#[derive(Clone)]
pub struct Signal<T> {
    id: SignalId,
    inner: Arc<Mutex<SignalInner<T>>>,
}

struct SignalInner<T> {
    value: T,
    listeners: Vec<SignalListener>,
}

impl<T: Clone> Signal<T> {
    /// Create a new reactive signal initialized with `value`.
    pub fn new(value: T) -> Self {
        Self {
            id: SignalId::next(),
            inner: Arc::new(Mutex::new(SignalInner {
                value,
                listeners: Vec::new(),
            })),
        }
    }

    fn lock_inner(&self) -> std::sync::MutexGuard<'_, SignalInner<T>> {
        match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// Read the signal's current value.
    pub fn get(&self) -> T {
        let inner = self.lock_inner();
        inner.value.clone()
    }

    /// Update the signal's value and notify all registered listeners.
    pub fn set(&self, new_value: T) {
        let listeners = {
            let mut inner = self.lock_inner();
            inner.value = new_value;
            inner.listeners.clone()
        };

        for listener in listeners {
            listener();
        }
    }

    /// Mutate the signal's value in-place via a closure.
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        let listeners = {
            let mut inner = self.lock_inner();
            f(&mut inner.value);
            inner.listeners.clone()
        };

        for listener in listeners {
            listener();
        }
    }

    /// Register a listener to be notified on subsequent mutations.
    pub fn subscribe(&self, listener: impl Fn() + Send + Sync + 'static) {
        let mut inner = self.lock_inner();
        inner.listeners.push(Arc::new(listener));
    }

    /// Unique ID for this signal.
    pub fn id(&self) -> SignalId {
        self.id
    }
}

/// Frame batch scheduler for coalescing dirty state notifications to vsync.
#[derive(Default)]
pub struct ReactiveBatch {
    dirty_signals: Mutex<Vec<SignalId>>,
    is_dirty: std::sync::atomic::AtomicBool,
}

impl ReactiveBatch {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark a signal as dirty for the upcoming frame pass.
    pub fn mark_dirty(&self, id: SignalId) {
        if let Ok(mut list) = self.dirty_signals.lock() {
            let already_present = list.contains(&id);
            if !already_present {
                list.push(id);
            }
        }
        self.is_dirty.store(true, Ordering::Release);
    }

    /// Check if any reactive state has changed since last frame.
    pub fn has_damage(&self) -> bool {
        self.is_dirty.load(Ordering::Acquire)
    }

    /// Consume all dirty signal IDs and reset damage state.
    pub fn drain_damage(&self) -> Vec<SignalId> {
        self.is_dirty.store(false, Ordering::Release);
        if let Ok(mut list) = self.dirty_signals.lock() {
            let drained = list.clone();
            list.clear();
            drained
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_get_set_and_listener() {
        let count = Signal::new(0);
        assert_eq!(count.get(), 0);

        let notified = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let notified_clone = Arc::clone(&notified);
        count.subscribe(move || {
            notified_clone.store(true, Ordering::SeqCst);
        });

        count.set(42);
        assert_eq!(count.get(), 42);
        assert!(notified.load(Ordering::SeqCst));
    }

    #[test]
    fn test_signal_update_closure() {
        let text = Signal::new("Hello".to_string());
        text.update(|s| s.push_str(" Agam!"));
        assert_eq!(text.get(), "Hello Agam!");
    }

    #[test]
    fn test_reactive_batch_coalescing() {
        let batch = ReactiveBatch::new();
        assert!(!batch.has_damage());

        let sig1 = SignalId::next();
        let sig2 = SignalId::next();

        batch.mark_dirty(sig1);
        batch.mark_dirty(sig2);
        batch.mark_dirty(sig1); // Duplicate should coalesce

        assert!(batch.has_damage());
        let drained = batch.drain_damage();
        assert_eq!(drained.len(), 2);
        assert!(!batch.has_damage());
    }
}
