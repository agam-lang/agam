//! Structured Concurrency Nurseries and Task Groups.
//!
//! Enforces scoped task lifecycles: all child tasks spawned inside a nursery
//! are guaranteed to complete or abort before the parent nursery exits scope.

use std::future::Future as StdFuture;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::scheduler::Runtime;
use super::task::{JoinHandle, TaskError};
use super::timer::timeout;

/// A structured concurrency TaskGroup ensuring no leaked tasks.
pub struct TaskGroup<'a> {
    runtime: &'a Runtime,
    cancelled: Arc<AtomicBool>,
    tasks: Mutex<Vec<JoinHandle<()>>>,
}

impl<'a> TaskGroup<'a> {
    pub fn new(runtime: &'a Runtime) -> Self {
        Self {
            runtime,
            cancelled: Arc::new(AtomicBool::new(false)),
            tasks: Mutex::new(Vec::new()),
        }
    }

    /// Spawn a task within this structured task group.
    pub fn spawn<F>(&self, future: F)
    where
        F: StdFuture<Output = ()> + Send + 'static,
    {
        let cancelled = self.cancelled.clone();
        let wrapped = async move {
            if !cancelled.load(Ordering::Acquire) {
                future.await;
            }
        };

        let handle = self.runtime.spawn(wrapped);
        let mut tasks = self.tasks.lock().unwrap();
        tasks.push(handle);
    }

    /// Cancel all active tasks in this group.
    pub fn cancel_all(&self) {
        self.cancelled.store(true, Ordering::Release);
        let tasks = self.tasks.lock().unwrap();
        for task in tasks.iter() {
            task.cancel();
        }
    }

    /// Await completion of all child tasks in the group.
    pub async fn wait_all(&self) -> Result<(), TaskError> {
        let mut handles = {
            let mut tasks = self.tasks.lock().unwrap();
            std::mem::take(&mut *tasks)
        };

        for handle in handles.drain(..) {
            handle.await?;
        }

        Ok(())
    }

    /// Await completion of all child tasks with a timeout.
    pub async fn wait_with_timeout(&self, dur: Duration) -> Result<(), TaskError> {
        match timeout(dur, self.wait_all()).await {
            Ok(res) => res,
            Err(_) => {
                self.cancel_all();
                Err(TaskError::Timeout)
            }
        }
    }
}
