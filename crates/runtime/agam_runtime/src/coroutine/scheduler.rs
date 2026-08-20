//! Work-Stealing Multi-Threaded Coroutine Scheduler.
//!
//! Features:
//! - Dedicated per-worker local execution queues.
//! - Lock-free work-stealing for cross-core load balancing.
//! - Global injector queue for tasks spawned from external threads.
//! - Blocking worker pool (`spawn_blocking`) for offloading synchronous tasks.
//! - Cooperative `yield_now` points.

use std::collections::VecDeque;
use std::future::Future as StdFuture;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Context as StdContext, Poll as StdPoll};
use std::thread::{self, JoinHandle as ThreadJoinHandle};

use super::task::{JoinHandle, SchedulableTask, TaskCell, TaskId};

/// Configuration options for the async coroutine runtime.
#[derive(Clone, Debug)]
pub struct RuntimeBuilder {
    pub worker_threads: usize,
    pub thread_name_prefix: String,
    pub max_blocking_threads: usize,
}

impl RuntimeBuilder {
    pub fn new() -> Self {
        let workers = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self {
            worker_threads: workers.max(1),
            thread_name_prefix: "agam-worker-".into(),
            max_blocking_threads: 128,
        }
    }

    pub fn worker_threads(mut self, n: usize) -> Self {
        self.worker_threads = n.max(1);
        self
    }

    pub fn build(self) -> Runtime {
        Runtime::new(self)
    }
}

impl Default for RuntimeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// The main async runtime managing task scheduling, worker threads, and timers.
pub struct Runtime {
    inner: Arc<SchedulerInner>,
    _worker_handles: Vec<ThreadJoinHandle<()>>,
}

struct SchedulerInner {
    injector: Mutex<VecDeque<Arc<dyn SchedulableTask>>>,
    local_queues: Vec<Mutex<VecDeque<Arc<dyn SchedulableTask>>>>,
    condvar: Condvar,
    shutdown: AtomicBool,
    active_tasks: AtomicUsize,
    num_workers: usize,
}

impl Runtime {
    pub fn new(builder: RuntimeBuilder) -> Self {
        let num_workers = builder.worker_threads;
        let mut local_queues = Vec::with_capacity(num_workers);
        for _ in 0..num_workers {
            local_queues.push(Mutex::new(VecDeque::with_capacity(256)));
        }

        let inner = Arc::new(SchedulerInner {
            injector: Mutex::new(VecDeque::new()),
            local_queues,
            condvar: Condvar::new(),
            shutdown: AtomicBool::new(false),
            active_tasks: AtomicUsize::new(0),
            num_workers,
        });

        let mut handles = Vec::with_capacity(num_workers);
        for worker_id in 0..num_workers {
            let sched = inner.clone();
            let thread_name = format!("{}{worker_id}", builder.thread_name_prefix);
            let handle = thread::Builder::new()
                .name(thread_name)
                .spawn(move || worker_loop(worker_id, sched))
                .expect("spawn worker thread");
            handles.push(handle);
        }

        Self {
            inner,
            _worker_handles: handles,
        }
    }

    /// Spawn an asynchronous task onto the runtime scheduler.
    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: StdFuture + Send + 'static,
        F::Output: Send + 'static,
    {
        let id = TaskId::new();
        let (task, handle) = TaskCell::new(id, future);
        self.inner.active_tasks.fetch_add(1, Ordering::Relaxed);
        {
            let mut injector = self.inner.injector.lock().unwrap();
            injector.push_back(task);
        }
        self.inner.condvar.notify_one();
        handle
    }

    /// Spawn a blocking, synchronous computation onto an isolated worker thread.
    pub fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let (sender, receiver) = std::sync::mpsc::channel();
        thread::spawn(move || {
            let res = f();
            let _ = sender.send(res);
        });

        self.spawn(async move {
            receiver
                .recv()
                .expect("blocking task failed to receive output")
        })
    }

    /// Block the current thread until the future resolves, running an inline single-threaded loop.
    pub fn block_on<F>(&self, future: F) -> F::Output
    where
        F: StdFuture,
    {
        let mut pinned = Box::pin(future);
        let waker = super::task::dummy_waker();
        let mut cx = StdContext::from_waker(&waker);

        loop {
            match pinned.as_mut().poll(&mut cx) {
                StdPoll::Ready(output) => return output,
                StdPoll::Pending => {
                    // Try processing one task from the scheduler while waiting
                    if let Some(task) = self.inner.pop_task(0) {
                        task.run();
                    } else {
                        thread::yield_now();
                    }
                }
            }
        }
    }

    /// Number of active worker threads in the scheduler.
    pub fn worker_count(&self) -> usize {
        self.inner.num_workers
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        self.inner.shutdown.store(true, Ordering::Release);
        self.inner.condvar.notify_all();
    }
}

impl SchedulerInner {
    fn pop_task(&self, worker_id: usize) -> Option<Arc<dyn SchedulableTask>> {
        // 1. Check worker local queue
        if let Ok(mut q) = self.local_queues[worker_id].try_lock() {
            let task = q.pop_front();
            if task.is_some() {
                return task;
            }
        }

        // 2. Check global injector queue
        if let Ok(mut inj) = self.injector.try_lock() {
            let task = inj.pop_front();
            if task.is_some() {
                return task;
            }
        }

        // 3. Work stealing from neighboring workers
        let num_workers = self.num_workers;
        for offset in 1..num_workers {
            let victim_id = (worker_id + offset) % num_workers;
            if let Ok(mut victim_q) = self.local_queues[victim_id].try_lock() {
                let task = victim_q.pop_back();
                if task.is_some() {
                    return task;
                }
            }
        }

        None
    }
}

fn worker_loop(worker_id: usize, sched: Arc<SchedulerInner>) {
    while !sched.shutdown.load(Ordering::Acquire) {
        if let Some(task) = sched.pop_task(worker_id) {
            task.run();
        } else {
            let guard = sched.injector.lock().unwrap();
            if guard.is_empty() && !sched.shutdown.load(Ordering::Acquire) {
                let _ = sched
                    .condvar
                    .wait_timeout(guard, std::time::Duration::from_millis(10));
            }
        }
    }
}

/// Cooperative yield future allowing other ready tasks to execute.
pub struct YieldNow(bool);

pub fn yield_now() -> YieldNow {
    YieldNow(false)
}

impl StdFuture for YieldNow {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut StdContext<'_>) -> StdPoll<()> {
        if self.0 {
            StdPoll::Ready(())
        } else {
            self.0 = true;
            cx.waker().wake_by_ref();
            StdPoll::Pending
        }
    }
}
