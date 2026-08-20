//! Fine-grained reactive state primitives (Signals, Computed, Effects, and Batching).

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_NODE_ID: AtomicU64 = AtomicU64::new(1);

type SubscriberCallback = Rc<RefCell<dyn FnMut()>>;

thread_local! {
    static CURRENT_OBSERVER: RefCell<Option<SubscriberCallback>> = const { RefCell::new(None) };
    static BATCH_DEPTH: RefCell<usize> = const { RefCell::new(0) };
    static PENDING_NOTIFICATIONS: RefCell<Vec<SubscriberCallback>> = const { RefCell::new(Vec::new()) };
}

/// A reactive signal holding a mutable value of type `T`.
#[derive(Clone)]
pub struct Signal<T> {
    id: u64,
    value: Rc<RefCell<T>>,
    subscribers: Rc<RefCell<Vec<SubscriberCallback>>>,
}

impl<T: Clone + 'static> Signal<T> {
    pub fn new(initial: T) -> Self {
        Self {
            id: NEXT_NODE_ID.fetch_add(1, Ordering::Relaxed),
            value: Rc::new(RefCell::new(initial)),
            subscribers: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    /// Read the current value and register dependency with active observer.
    pub fn get(&self) -> T {
        CURRENT_OBSERVER.with(|obs| {
            if let Some(subscriber) = obs.borrow().as_ref() {
                let mut subs = self.subscribers.borrow_mut();
                if !subs.iter().any(|s| Rc::ptr_eq(s, subscriber)) {
                    subs.push(Rc::clone(subscriber));
                }
            }
        });
        self.value.borrow().clone()
    }

    /// Update the signal's value and notify all subscribers.
    pub fn set(&self, new_val: T) {
        *self.value.borrow_mut() = new_val;
        self.notify_subscribers();
    }

    /// Update the value via closure in-place.
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        f(&mut self.value.borrow_mut());
        self.notify_subscribers();
    }

    fn notify_subscribers(&self) {
        let subs = self.subscribers.borrow().clone();
        BATCH_DEPTH.with(|depth| {
            if *depth.borrow() > 0 {
                PENDING_NOTIFICATIONS.with(|pending| {
                    pending.borrow_mut().extend(subs);
                });
            } else {
                for sub in subs {
                    (sub.borrow_mut())();
                }
            }
        });
    }
}

/// A derived, memoized reactive computation.
#[derive(Clone)]
pub struct Computed<T> {
    signal: Signal<T>,
    _effect: Rc<RefCell<dyn FnMut()>>,
}

impl<T: Clone + PartialEq + 'static> Computed<T> {
    pub fn new<F: Fn() -> T + 'static>(compute: F) -> Self {
        let compute_rc = Rc::new(compute);
        let initial = compute_rc();
        let signal = Signal::new(initial);

        let sig_clone = signal.clone();
        let comp_clone = Rc::clone(&compute_rc);

        let runner: Rc<RefCell<dyn FnMut()>> = Rc::new(RefCell::new(move || {
            let next_val = comp_clone();
            if sig_clone.get() != next_val {
                sig_clone.set(next_val);
            }
        }));

        let runner_clone = Rc::clone(&runner);
        CURRENT_OBSERVER.with(|obs| {
            let prev = obs.borrow_mut().take();
            *obs.borrow_mut() = Some(runner_clone);
            let _ = compute_rc();
            *obs.borrow_mut() = prev;
        });

        Self {
            signal,
            _effect: runner,
        }
    }

    pub fn get(&self) -> T {
        self.signal.get()
    }
}

/// Run an effect closure whenever its dependencies change.
pub fn create_effect<F: FnMut() + 'static>(mut effect: F) {
    let runner: Rc<RefCell<dyn FnMut()>> = Rc::new(RefCell::new(move || {
        effect();
    }));

    let runner_clone = Rc::clone(&runner);
    CURRENT_OBSERVER.with(|obs| {
        let prev = obs.borrow_mut().take();
        *obs.borrow_mut() = Some(runner_clone);
        (runner.borrow_mut())();
        *obs.borrow_mut() = prev;
    });
}

/// Execute a closure inside a batch transaction.
///
/// Postpones all subscriber notifications until the batch finishes,
/// preventing intermediate layout thrashing.
pub fn batch<R, F: FnOnce() -> R>(f: F) -> R {
    BATCH_DEPTH.with(|depth| {
        *depth.borrow_mut() += 1;
    });

    let res = f();

    let flush = BATCH_DEPTH.with(|depth| {
        let mut d = depth.borrow_mut();
        *d = d.saturating_sub(1);
        *d == 0
    });

    if flush {
        let pending = PENDING_NOTIFICATIONS.with(|p| p.borrow_mut().split_off(0));
        for sub in pending {
            (sub.borrow_mut())();
        }
    }

    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_basic_get_set() {
        let count = Signal::new(0);
        assert_eq!(count.get(), 0);
        count.set(42);
        assert_eq!(count.get(), 42);
    }

    #[test]
    fn test_computed_signal_updates() {
        let count = Signal::new(2);
        let count_clone = count.clone();
        let double = Computed::new(move || count_clone.get() * 2);

        assert_eq!(double.get(), 4);
        count.set(10);
        assert_eq!(double.get(), 20);
    }

    #[test]
    fn test_effect_reactivity() {
        let count = Signal::new(1);
        let count_clone = count.clone();
        let observed = Rc::new(RefCell::new(0));
        let observed_clone = Rc::clone(&observed);

        create_effect(move || {
            *observed_clone.borrow_mut() = count_clone.get();
        });

        assert_eq!(*observed.borrow(), 1);
        count.set(5);
        assert_eq!(*observed.borrow(), 5);
    }

    #[test]
    fn test_batch_execution() {
        let a = Signal::new(1);
        let b = Signal::new(2);
        let runs = Rc::new(RefCell::new(0));

        let a_c = a.clone();
        let b_c = b.clone();
        let runs_c = Rc::clone(&runs);

        create_effect(move || {
            let _ = a_c.get() + b_c.get();
            *runs_c.borrow_mut() += 1;
        });

        assert_eq!(*runs.borrow(), 1);

        batch(|| {
            a.set(10);
            b.set(20);
        });

        assert_eq!(*runs.borrow(), 3); // 1 initial + 2 queued in batch
    }
}
