//! First-party multi-core concurrency and synchronization utilities with Zero GIL.
//!
//! Provides native thread spawning, parallel chunk iteration, thread-safe channels,
//! and mutual exclusion containers per `ADOPTED_DEPENDENCIES.md` and `note.md`.

#![deny(clippy::unwrap_used)]

use std::fmt;
use std::sync::mpsc::{Receiver as StdReceiver, Sender as StdSender, channel as std_channel};
use std::sync::{Arc, Mutex as StdMutex};
use std::thread::{JoinHandle, available_parallelism, spawn as std_spawn};

/// Structured concurrency diagnostic formatted in the Agam Nyāya voice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncError {
    pub cause: String,
    pub context: String,
    pub remedy: String,
}

impl SyncError {
    pub fn new(
        cause: impl fmt::Display,
        context: impl fmt::Display,
        remedy: impl fmt::Display,
    ) -> Self {
        Self {
            cause: cause.to_string(),
            context: context.to_string(),
            remedy: remedy.to_string(),
        }
    }
}

impl fmt::Display for SyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Sync Diagnostic: {}\n  Context: {}\n  Remedy:  {}",
            self.cause, self.context, self.remedy
        )
    }
}

impl std::error::Error for SyncError {}

/// Spawn a new native OS thread executing the closure `f` with zero GIL overhead.
pub fn spawn<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    std_spawn(f)
}

/// Execute a parallel for-loop across the index range `[start, end)` partitioned across CPU cores.
pub fn parallel_for<F>(start: usize, end: usize, f: F)
where
    F: Fn(usize) + Sync + Send + 'static,
{
    if start >= end {
        return;
    }
    let total = end - start;
    let num_threads = available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(total)
        .max(1);

    let chunk_size = total.div_ceil(num_threads);
    let f_arc = Arc::new(f);
    let mut handles = Vec::with_capacity(num_threads);

    for t in 0..num_threads {
        let chunk_start = start + t * chunk_size;
        let chunk_end = (chunk_start + chunk_size).min(end);
        if chunk_start >= chunk_end {
            continue;
        }
        let f_clone = Arc::clone(&f_arc);
        let handle = std_spawn(move || {
            for i in chunk_start..chunk_end {
                f_clone(i);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.join();
    }
}

/// Thread-safe message sender.
pub struct Sender<T> {
    inner: StdSender<T>,
}

impl<T> Sender<T> {
    /// Send a message across the channel.
    pub fn send(&self, val: T) -> Result<(), SyncError> {
        self.inner.send(val).map_err(|e| {
            SyncError::new(
                format!("Failed to send message: {}", e),
                "Channel receiver has disconnected or closed",
                "Ensure receiver remains alive while sending messages",
            )
        })
    }
}

/// Thread-safe message receiver.
pub struct Receiver<T> {
    inner: StdReceiver<T>,
}

impl<T> Receiver<T> {
    /// Blocking receive of next message.
    pub fn recv(&self) -> Result<T, SyncError> {
        self.inner.recv().map_err(|e| {
            SyncError::new(
                format!("Failed to receive message: {}", e),
                "Channel sender has disconnected or closed",
                "Ensure sender produces messages or handle EOF cleanly",
            )
        })
    }

    /// Non-blocking receive of next message.
    pub fn try_recv(&self) -> Result<Option<T>, SyncError> {
        match self.inner.try_recv() {
            Ok(v) => Ok(Some(v)),
            Err(std::sync::mpsc::TryRecvError::Empty) => Ok(None),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => Err(SyncError::new(
                "Channel disconnected during try_recv",
                "All sender handles were dropped",
                "Verify channel producer lifecycle",
            )),
        }
    }
}

/// Create a new thread-safe MPSC channel returning a `(Sender, Receiver)` pair.
pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let (tx, rx) = std_channel();
    (Sender { inner: tx }, Receiver { inner: rx })
}

/// Thread-safe mutual exclusion wrapper.
#[derive(Debug)]
pub struct Mutex<T> {
    inner: StdMutex<T>,
}

impl<T> Mutex<T> {
    /// Construct a new mutex protecting `val`.
    pub fn new(val: T) -> Self {
        Self {
            inner: StdMutex::new(val),
        }
    }

    /// Access protected state inside closure `f` under mutex acquisition.
    pub fn with_lock<R, F: FnOnce(&mut T) -> R>(&self, f: F) -> Result<R, SyncError> {
        match self.inner.lock() {
            Ok(mut guard) => Ok(f(&mut *guard)),
            Err(poisoned) => Err(SyncError::new(
                "Mutex is poisoned due to previous thread panic",
                format!("Mutex state: {:?}", poisoned),
                "Handle thread crashes gracefully before acquiring poisoned lock",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_spawn_and_join() {
        let handle = spawn(|| 40 + 2);
        let res = handle.join();
        assert!(res.is_ok());
        if let Ok(val) = res {
            assert_eq!(val, 42);
        }
    }

    #[test]
    fn test_parallel_for_multi_core_accumulation() {
        let sum = Arc::new(AtomicUsize::new(0));
        let sum_clone = Arc::clone(&sum);

        parallel_for(1, 101, move |i| {
            sum_clone.fetch_add(i, Ordering::SeqCst);
        });

        // Sum of 1..=100 = 5050
        assert_eq!(sum.load(Ordering::SeqCst), 5050);
    }

    #[test]
    fn test_channel_send_recv() {
        let (tx, rx) = channel();
        let handle = spawn(move || {
            let _ = tx.send("agam_message");
        });

        let msg = rx.recv();
        assert!(msg.is_ok());
        if let Ok(m) = msg {
            assert_eq!(m, "agam_message");
        }
        let _ = handle.join();
    }

    #[test]
    fn test_mutex_with_lock() {
        let m = Mutex::new(10);
        let res = m.with_lock(|val| {
            *val += 5;
            *val
        });
        assert!(res.is_ok());
        if let Ok(v) = res {
            assert_eq!(v, 15);
        }
    }
}
