//! Task and Future abstractions for the Agam stackless coroutine runtime.

use std::fmt;
use std::future::Future as StdFuture;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{
    Context as StdContext, Poll as StdPoll, RawWaker, RawWakerVTable, Waker as StdWaker,
};

static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);

/// Unique identifier for an asynchronous task.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(pub u64);

impl TaskId {
    pub fn new() -> Self {
        Self(NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Task({})", self.0)
    }
}

/// Status of an asynchronous poll operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Poll<T> {
    /// The computation is complete and ready.
    Ready(T),
    /// The computation is suspended and pending a wake event.
    Pending,
}

impl<T> Poll<T> {
    pub fn is_ready(&self) -> bool {
        matches!(self, Poll::Ready(_))
    }

    pub fn is_pending(&self) -> bool {
        matches!(self, Poll::Pending)
    }

    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Poll<U> {
        match self {
            Poll::Ready(val) => Poll::Ready(f(val)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T> From<StdPoll<T>> for Poll<T> {
    fn from(p: StdPoll<T>) -> Self {
        match p {
            StdPoll::Ready(v) => Poll::Ready(v),
            StdPoll::Pending => Poll::Pending,
        }
    }
}

impl<T> From<Poll<T>> for StdPoll<T> {
    fn from(p: Poll<T>) -> Self {
        match p {
            Poll::Ready(v) => StdPoll::Ready(v),
            Poll::Pending => StdPoll::Pending,
        }
    }
}

/// A handle to a spawned async task that can be awaited to retrieve its output.
pub struct JoinHandle<T> {
    pub id: TaskId,
    state: Arc<Mutex<TaskState<T>>>,
    cancelled: Arc<AtomicBool>,
}

#[allow(dead_code)]
pub(crate) enum TaskState<T> {
    Running,
    Finished(T),
    Cancelled,
    Panicked(String),
}

impl<T> JoinHandle<T> {
    pub(crate) fn new(id: TaskId, cancelled: Arc<AtomicBool>) -> (Self, Arc<Mutex<TaskState<T>>>) {
        let state = Arc::new(Mutex::new(TaskState::Running));
        let handle = Self {
            id,
            state: state.clone(),
            cancelled,
        };
        (handle, state)
    }

    /// Cancel the associated task.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        let mut state = self.state.lock().unwrap();
        if let TaskState::Running = *state {
            *state = TaskState::Cancelled;
        }
    }

    /// Returns true if the task was cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    /// Returns true if the task has completed execution.
    pub fn is_finished(&self) -> bool {
        let state = self.state.lock().unwrap();
        !matches!(*state, TaskState::Running)
    }
}

impl<T: Send + 'static> StdFuture for JoinHandle<T> {
    type Output = Result<T, TaskError>;

    fn poll(self: Pin<&mut Self>, _cx: &mut StdContext<'_>) -> StdPoll<Self::Output> {
        let mut state = self.state.lock().unwrap();
        match &*state {
            TaskState::Running => StdPoll::Pending,
            TaskState::Finished(_) => {
                if let TaskState::Finished(val) =
                    std::mem::replace(&mut *state, TaskState::Cancelled)
                {
                    StdPoll::Ready(Ok(val))
                } else {
                    unreachable!()
                }
            }
            TaskState::Cancelled => StdPoll::Ready(Err(TaskError::Cancelled)),
            TaskState::Panicked(msg) => StdPoll::Ready(Err(TaskError::Panicked(msg.clone()))),
        }
    }
}

/// Errors occurring during task execution or joining.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskError {
    Cancelled,
    Panicked(String),
    Timeout,
}

impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskError::Cancelled => write!(f, "task was cancelled"),
            TaskError::Panicked(msg) => write!(f, "task panicked: {msg}"),
            TaskError::Timeout => write!(f, "task timed out"),
        }
    }
}

impl std::error::Error for TaskError {}

/// An executable task unit managed by the runtime scheduler.
pub trait SchedulableTask: Send + Sync {
    fn id(&self) -> TaskId;
    fn run(&self);
    fn is_cancelled(&self) -> bool;
}

pub(crate) struct TaskCell<F: StdFuture> {
    pub id: TaskId,
    pub future: Mutex<Option<Pin<Box<F>>>>,
    pub state: Arc<Mutex<TaskState<F::Output>>>,
    pub cancelled: Arc<AtomicBool>,
}

impl<F> TaskCell<F>
where
    F: StdFuture + Send + 'static,
    F::Output: Send + 'static,
{
    pub fn new(id: TaskId, future: F) -> (Arc<Self>, JoinHandle<F::Output>) {
        let cancelled = Arc::new(AtomicBool::new(false));
        let (handle, state) = JoinHandle::new(id, cancelled.clone());
        let cell = Arc::new(Self {
            id,
            future: Mutex::new(Some(Box::pin(future))),
            state,
            cancelled,
        });
        (cell, handle)
    }
}

impl<F> SchedulableTask for TaskCell<F>
where
    F: StdFuture + Send + 'static,
    F::Output: Send + 'static,
{
    fn id(&self) -> TaskId {
        self.id
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    fn run(&self) {
        if self.is_cancelled() {
            let mut state = self.state.lock().unwrap();
            *state = TaskState::Cancelled;
            return;
        }

        let mut future_guard = self.future.lock().unwrap();
        if let Some(mut fut) = future_guard.take() {
            let waker = dummy_waker();
            let mut cx = StdContext::from_waker(&waker);

            match fut.as_mut().poll(&mut cx) {
                StdPoll::Ready(output) => {
                    let mut state = self.state.lock().unwrap();
                    *state = TaskState::Finished(output);
                }
                StdPoll::Pending => {
                    *future_guard = Some(fut);
                }
            }
        }
    }
}

pub(crate) fn dummy_waker() -> StdWaker {
    static VTABLE: RawWakerVTable = RawWakerVTable::new(
        |_| RawWaker::new(std::ptr::null(), &VTABLE),
        |_| {},
        |_| {},
        |_| {},
    );
    unsafe { StdWaker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) }
}
