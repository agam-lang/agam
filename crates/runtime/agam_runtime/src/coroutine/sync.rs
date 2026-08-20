//! Async-aware Synchronization Primitives.
//!
//! Includes AsyncMutex, AsyncSemaphore, AsyncBarrier.

use std::cell::UnsafeCell;
use std::collections::VecDeque;
use std::future::Future as StdFuture;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::Mutex;
use std::task::{Context as StdContext, Poll as StdPoll, Waker};

// ══════════════════════════════════════════════════════════════════════
// 1. Async Mutex
// ══════════════════════════════════════════════════════════════════════

/// An asynchronous mutual exclusion lock for protecting shared mutable data across tasks.
pub struct AsyncMutex<T: ?Sized> {
    state: Mutex<MutexState>,
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for AsyncMutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for AsyncMutex<T> {}

struct MutexState {
    locked: bool,
    waiters: VecDeque<Waker>,
}

impl<T> AsyncMutex<T> {
    pub fn new(data: T) -> Self {
        Self {
            state: Mutex::new(MutexState {
                locked: false,
                waiters: VecDeque::new(),
            }),
            data: UnsafeCell::new(data),
        }
    }

    /// Asynchronously acquires the lock, suspending until available.
    pub async fn lock(&self) -> AsyncMutexGuard<'_, T> {
        struct LockFuture<'a, T: ?Sized> {
            mutex: &'a AsyncMutex<T>,
        }

        impl<'a, T: ?Sized> StdFuture for LockFuture<'a, T> {
            type Output = AsyncMutexGuard<'a, T>;

            fn poll(self: Pin<&mut Self>, cx: &mut StdContext<'_>) -> StdPoll<Self::Output> {
                let mut state = self.mutex.state.lock().unwrap();
                if !state.locked {
                    state.locked = true;
                    StdPoll::Ready(AsyncMutexGuard { mutex: self.mutex })
                } else {
                    state.waiters.push_back(cx.waker().clone());
                    StdPoll::Pending
                }
            }
        }

        LockFuture { mutex: self }.await
    }

    /// Attempts to acquire the lock immediately without suspending.
    pub fn try_lock(&self) -> Option<AsyncMutexGuard<'_, T>> {
        let mut state = self.state.lock().unwrap();
        if !state.locked {
            state.locked = true;
            Some(AsyncMutexGuard { mutex: self })
        } else {
            None
        }
    }
}

pub struct AsyncMutexGuard<'a, T: ?Sized> {
    mutex: &'a AsyncMutex<T>,
}

impl<'a, T: ?Sized> Deref for AsyncMutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<'a, T: ?Sized> DerefMut for AsyncMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<'a, T: ?Sized> Drop for AsyncMutexGuard<'a, T> {
    fn drop(&mut self) {
        let mut state = self.mutex.state.lock().unwrap();
        if let Some(waker) = state.waiters.pop_front() {
            waker.wake();
        } else {
            state.locked = false;
        }
    }
}

// ══════════════════════════════════════════════════════════════════════
// 2. Async Semaphore
// ══════════════════════════════════════════════════════════════════════

/// An asynchronous counting semaphore.
pub struct AsyncSemaphore {
    state: Mutex<SemaphoreState>,
}

struct SemaphoreState {
    permits: usize,
    waiters: VecDeque<(usize, Waker)>,
}

impl AsyncSemaphore {
    pub fn new(permits: usize) -> Self {
        Self {
            state: Mutex::new(SemaphoreState {
                permits,
                waiters: VecDeque::new(),
            }),
        }
    }

    pub async fn acquire(&self) -> SemaphorePermit<'_> {
        self.acquire_many(1).await
    }

    pub async fn acquire_many(&self, count: usize) -> SemaphorePermit<'_> {
        struct AcquireFuture<'a> {
            sem: &'a AsyncSemaphore,
            count: usize,
        }

        impl<'a> StdFuture for AcquireFuture<'a> {
            type Output = SemaphorePermit<'a>;

            fn poll(self: Pin<&mut Self>, cx: &mut StdContext<'_>) -> StdPoll<Self::Output> {
                let mut state = self.sem.state.lock().unwrap();
                if state.permits >= self.count {
                    state.permits -= self.count;
                    StdPoll::Ready(SemaphorePermit {
                        sem: self.sem,
                        count: self.count,
                    })
                } else {
                    state.waiters.push_back((self.count, cx.waker().clone()));
                    StdPoll::Pending
                }
            }
        }

        AcquireFuture { sem: self, count }.await
    }

    pub fn available_permits(&self) -> usize {
        self.state.lock().unwrap().permits
    }
}

pub struct SemaphorePermit<'a> {
    sem: &'a AsyncSemaphore,
    count: usize,
}

impl<'a> Drop for SemaphorePermit<'a> {
    fn drop(&mut self) {
        let mut state = self.sem.state.lock().unwrap();
        state.permits += self.count;

        while let Some((needed, _)) = state.waiters.front() {
            if state.permits >= *needed {
                let (_, waker) = state.waiters.pop_front().unwrap();
                waker.wake();
            } else {
                break;
            }
        }
    }
}

// ══════════════════════════════════════════════════════════════════════
// 3. Async Barrier
// ══════════════════════════════════════════════════════════════════════

/// An asynchronous synchronization barrier enabling multiple tasks to wait for each other.
pub struct AsyncBarrier {
    threshold: usize,
    state: Mutex<BarrierState>,
}

struct BarrierState {
    count: usize,
    generation: usize,
    waiters: Vec<Waker>,
}

impl AsyncBarrier {
    pub fn new(threshold: usize) -> Self {
        Self {
            threshold,
            state: Mutex::new(BarrierState {
                count: 0,
                generation: 0,
                waiters: Vec::with_capacity(threshold),
            }),
        }
    }

    pub async fn wait(&self) -> bool {
        struct BarrierFuture<'a> {
            barrier: &'a AsyncBarrier,
            generation: usize,
            registered: bool,
        }

        impl<'a> StdFuture for BarrierFuture<'a> {
            type Output = bool;

            fn poll(mut self: Pin<&mut Self>, cx: &mut StdContext<'_>) -> StdPoll<Self::Output> {
                let mut state = self.barrier.state.lock().unwrap();

                if !self.registered {
                    self.registered = true;
                    self.generation = state.generation;
                    state.count += 1;

                    if state.count == self.barrier.threshold {
                        state.count = 0;
                        state.generation += 1;
                        for waker in state.waiters.drain(..) {
                            waker.wake();
                        }
                        return StdPoll::Ready(true); // Leader
                    } else {
                        state.waiters.push(cx.waker().clone());
                        return StdPoll::Pending;
                    }
                }

                if state.generation != self.generation {
                    StdPoll::Ready(false)
                } else {
                    StdPoll::Pending
                }
            }
        }

        BarrierFuture {
            barrier: self,
            generation: 0,
            registered: false,
        }
        .await
    }
}
