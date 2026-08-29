//! Async-aware Synchronization Primitives.
//!
//! Includes AsyncMutex, AsyncRwLock, AsyncCondvar, AsyncSemaphore, AsyncBarrier.

use std::cell::UnsafeCell;
use std::collections::VecDeque;
use std::future::Future as StdFuture;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
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
        state.locked = false;
        if let Some(waker) = state.waiters.pop_front() {
            waker.wake();
        }
    }
}

// ══════════════════════════════════════════════════════════════════════
// 2. Async Read-Write Lock (AsyncRwLock)
// ══════════════════════════════════════════════════════════════════════

/// An asynchronous reader-writer lock enabling multiple concurrent readers or an exclusive writer.
pub struct AsyncRwLock<T: ?Sized> {
    state: Mutex<RwLockState>,
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for AsyncRwLock<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for AsyncRwLock<T> {}

struct RwLockState {
    readers: usize,
    writer: bool,
    read_waiters: VecDeque<Waker>,
    write_waiters: VecDeque<Waker>,
}

impl<T> AsyncRwLock<T> {
    pub fn new(data: T) -> Self {
        Self {
            state: Mutex::new(RwLockState {
                readers: 0,
                writer: false,
                read_waiters: VecDeque::new(),
                write_waiters: VecDeque::new(),
            }),
            data: UnsafeCell::new(data),
        }
    }

    /// Asynchronously acquires shared read access.
    pub async fn read(&self) -> AsyncRwLockReadGuard<'_, T> {
        struct ReadFuture<'a, T: ?Sized> {
            lock: &'a AsyncRwLock<T>,
        }

        impl<'a, T: ?Sized> StdFuture for ReadFuture<'a, T> {
            type Output = AsyncRwLockReadGuard<'a, T>;

            fn poll(self: Pin<&mut Self>, cx: &mut StdContext<'_>) -> StdPoll<Self::Output> {
                let mut state = self.lock.state.lock().unwrap();
                if !state.writer {
                    state.readers += 1;
                    StdPoll::Ready(AsyncRwLockReadGuard { lock: self.lock })
                } else {
                    state.read_waiters.push_back(cx.waker().clone());
                    StdPoll::Pending
                }
            }
        }

        ReadFuture { lock: self }.await
    }

    /// Asynchronously acquires exclusive write access.
    pub async fn write(&self) -> AsyncRwLockWriteGuard<'_, T> {
        struct WriteFuture<'a, T: ?Sized> {
            lock: &'a AsyncRwLock<T>,
        }

        impl<'a, T: ?Sized> StdFuture for WriteFuture<'a, T> {
            type Output = AsyncRwLockWriteGuard<'a, T>;

            fn poll(self: Pin<&mut Self>, cx: &mut StdContext<'_>) -> StdPoll<Self::Output> {
                let mut state = self.lock.state.lock().unwrap();
                if !state.writer && state.readers == 0 {
                    state.writer = true;
                    StdPoll::Ready(AsyncRwLockWriteGuard { lock: self.lock })
                } else {
                    state.write_waiters.push_back(cx.waker().clone());
                    StdPoll::Pending
                }
            }
        }

        WriteFuture { lock: self }.await
    }
}

pub struct AsyncRwLockReadGuard<'a, T: ?Sized> {
    lock: &'a AsyncRwLock<T>,
}

impl<'a, T: ?Sized> Deref for AsyncRwLockReadGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T: ?Sized> Drop for AsyncRwLockReadGuard<'a, T> {
    fn drop(&mut self) {
        let mut state = self.lock.state.lock().unwrap();
        state.readers = state.readers.saturating_sub(1);
        if state.readers == 0 {
            let next_waker = state.write_waiters.pop_front();
            if let Some(waker) = next_waker {
                waker.wake();
            }
        }
    }
}

pub struct AsyncRwLockWriteGuard<'a, T: ?Sized> {
    lock: &'a AsyncRwLock<T>,
}

impl<'a, T: ?Sized> Deref for AsyncRwLockWriteGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T: ?Sized> DerefMut for AsyncRwLockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T: ?Sized> Drop for AsyncRwLockWriteGuard<'a, T> {
    fn drop(&mut self) {
        let mut state = self.lock.state.lock().unwrap();
        state.writer = false;
        if let Some(waker) = state.write_waiters.pop_front() {
            waker.wake();
        } else {
            for waker in state.read_waiters.drain(..) {
                waker.wake();
            }
        }
    }
}

// ══════════════════════════════════════════════════════════════════════
// 3. Async Condition Variable (AsyncCondvar)
// ══════════════════════════════════════════════════════════════════════

/// An asynchronous condition variable enabling tasks to wait for specific predicates.
pub struct AsyncCondvar {
    waiters: Mutex<VecDeque<(Arc<AtomicBool>, Option<Waker>)>>,
}

impl AsyncCondvar {
    pub fn new() -> Self {
        Self {
            waiters: Mutex::new(VecDeque::new()),
        }
    }

    pub fn notify_one(&self) {
        let mut waiters = self.waiters.lock().unwrap();
        if let Some((notified, maybe_waker)) = waiters.pop_front() {
            notified.store(true, Ordering::Release);
            if let Some(waker) = maybe_waker {
                waker.wake();
            }
        }
    }

    pub fn notify_all(&self) {
        let mut waiters = self.waiters.lock().unwrap();
        for (notified, maybe_waker) in waiters.drain(..) {
            notified.store(true, Ordering::Release);
            if let Some(waker) = maybe_waker {
                waker.wake();
            }
        }
    }

    pub async fn wait<'a, T>(&self, guard: AsyncMutexGuard<'a, T>) -> AsyncMutexGuard<'a, T> {
        let mutex = guard.mutex;
        let notified = Arc::new(AtomicBool::new(false));

        // Enqueue BEFORE releasing the mutex guard
        {
            let mut waiters = self.waiters.lock().unwrap();
            waiters.push_back((notified.clone(), None));
        }

        // Release mutex guard
        drop(guard);

        struct WaitFut<'a> {
            condvar: &'a AsyncCondvar,
            notified: Arc<AtomicBool>,
        }

        impl<'a> StdFuture for WaitFut<'a> {
            type Output = ();

            fn poll(self: Pin<&mut Self>, cx: &mut StdContext<'_>) -> StdPoll<Self::Output> {
                if self.notified.load(Ordering::Acquire) {
                    return StdPoll::Ready(());
                }

                let mut waiters = self.condvar.waiters.lock().unwrap();
                if self.notified.load(Ordering::Acquire) {
                    return StdPoll::Ready(());
                }

                for (flag, waker_slot) in waiters.iter_mut() {
                    if Arc::ptr_eq(flag, &self.notified) {
                        *waker_slot = Some(cx.waker().clone());
                        break;
                    }
                }

                StdPoll::Pending
            }
        }

        WaitFut {
            condvar: self,
            notified,
        }
        .await;

        mutex.lock().await
    }
}

impl Default for AsyncCondvar {
    fn default() -> Self {
        Self::new()
    }
}

// ══════════════════════════════════════════════════════════════════════
// 4. Async Semaphore
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
// 5. Async Barrier
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
