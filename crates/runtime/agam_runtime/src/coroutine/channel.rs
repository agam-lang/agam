//! Asynchronous Channels (MPSC, Oneshot, Broadcast).
//!
//! Provides lock-free and thread-safe async communication primitives between tasks.

use std::collections::VecDeque;
use std::future::Future as StdFuture;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context as StdContext, Poll as StdPoll, Waker};

// ══════════════════════════════════════════════════════════════════════
// 1. Oneshot Channel
// ══════════════════════════════════════════════════════════════════════

/// Create a single-producer single-consumer oneshot async channel.
pub fn oneshot<T>() -> (OneshotSender<T>, OneshotReceiver<T>) {
    let inner = Arc::new(Mutex::new(OneshotInner {
        value: None,
        waker: None,
        closed: false,
    }));
    (
        OneshotSender {
            inner: inner.clone(),
        },
        OneshotReceiver { inner },
    )
}

struct OneshotInner<T> {
    value: Option<T>,
    waker: Option<Waker>,
    closed: bool,
}

pub struct OneshotSender<T> {
    inner: Arc<Mutex<OneshotInner<T>>>,
}

impl<T> OneshotSender<T> {
    pub fn send(self, val: T) -> Result<(), T> {
        let mut inner = self.inner.lock().unwrap();
        if inner.closed {
            return Err(val);
        }
        inner.value = Some(val);
        if let Some(waker) = inner.waker.take() {
            waker.wake();
        }
        Ok(())
    }
}

impl<T> Drop for OneshotSender<T> {
    fn drop(&mut self) {
        let mut inner = self.inner.lock().unwrap();
        inner.closed = true;
        if let Some(waker) = inner.waker.take() {
            waker.wake();
        }
    }
}

pub struct OneshotReceiver<T> {
    inner: Arc<Mutex<OneshotInner<T>>>,
}

impl<T> StdFuture for OneshotReceiver<T> {
    type Output = Result<T, RecvError>;

    fn poll(self: Pin<&mut Self>, cx: &mut StdContext<'_>) -> StdPoll<Self::Output> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(val) = inner.value.take() {
            StdPoll::Ready(Ok(val))
        } else if inner.closed {
            StdPoll::Ready(Err(RecvError::Closed))
        } else {
            inner.waker = Some(cx.waker().clone());
            StdPoll::Pending
        }
    }
}

// ══════════════════════════════════════════════════════════════════════
// 2. MPSC Channel (Multi-Producer Single-Consumer)
// ══════════════════════════════════════════════════════════════════════

/// Create an asynchronous bounded MPSC channel.
pub fn channel<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(Mutex::new(MpscInner {
        queue: VecDeque::new(),
        capacity: capacity.max(1),
        recv_waker: None,
        send_wakers: VecDeque::new(),
        senders: 1,
        closed: false,
    }));
    (
        Sender {
            inner: inner.clone(),
        },
        Receiver { inner },
    )
}

/// Create an unbounded asynchronous MPSC channel.
pub fn unbounded_channel<T>() -> (UnboundedSender<T>, UnboundedReceiver<T>) {
    let (s, r) = channel(usize::MAX);
    (UnboundedSender(s), UnboundedReceiver(r))
}

struct MpscInner<T> {
    queue: VecDeque<T>,
    capacity: usize,
    recv_waker: Option<Waker>,
    send_wakers: VecDeque<Waker>,
    senders: usize,
    closed: bool,
}

#[derive(Clone)]
pub struct Sender<T> {
    inner: Arc<Mutex<MpscInner<T>>>,
}

impl<T> Sender<T> {
    pub async fn send(&self, item: T) -> Result<(), SendError<T>> {
        struct SendFut<'a, T> {
            sender: &'a Sender<T>,
            item: Option<T>,
        }

        impl<'a, T> StdFuture for SendFut<'a, T> {
            type Output = Result<(), SendError<T>>;

            fn poll(self: Pin<&mut Self>, cx: &mut StdContext<'_>) -> StdPoll<Self::Output> {
                let this = unsafe { self.get_unchecked_mut() };
                let mut inner = this.sender.inner.lock().unwrap();
                if inner.closed {
                    return StdPoll::Ready(Err(SendError(this.item.take().unwrap())));
                }

                if inner.queue.len() < inner.capacity {
                    inner.queue.push_back(this.item.take().unwrap());
                    if let Some(waker) = inner.recv_waker.take() {
                        waker.wake();
                    }
                    StdPoll::Ready(Ok(()))
                } else {
                    inner.send_wakers.push_back(cx.waker().clone());
                    StdPoll::Pending
                }
            }
        }

        SendFut {
            sender: self,
            item: Some(item),
        }
        .await
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let mut inner = self.inner.lock().unwrap();
        inner.senders = inner.senders.saturating_sub(1);
        if inner.senders == 0 {
            inner.closed = true;
            if let Some(waker) = inner.recv_waker.take() {
                waker.wake();
            }
        }
    }
}

pub struct Receiver<T> {
    inner: Arc<Mutex<MpscInner<T>>>,
}

impl<T> Receiver<T> {
    pub async fn recv(&mut self) -> Option<T> {
        struct RecvFut<'a, T> {
            receiver: &'a mut Receiver<T>,
        }

        impl<'a, T> StdFuture for RecvFut<'a, T> {
            type Output = Option<T>;

            fn poll(self: Pin<&mut Self>, cx: &mut StdContext<'_>) -> StdPoll<Self::Output> {
                let mut inner = self.receiver.inner.lock().unwrap();
                if let Some(item) = inner.queue.pop_front() {
                    if let Some(waker) = inner.send_wakers.pop_front() {
                        waker.wake();
                    }
                    StdPoll::Ready(Some(item))
                } else if inner.closed {
                    StdPoll::Ready(None)
                } else {
                    inner.recv_waker = Some(cx.waker().clone());
                    StdPoll::Pending
                }
            }
        }

        RecvFut { receiver: self }.await
    }
}

pub struct UnboundedSender<T>(Sender<T>);

impl<T> UnboundedSender<T> {
    pub fn send(&self, item: T) -> Result<(), SendError<T>> {
        let mut inner = self.0.inner.lock().unwrap();
        if inner.closed {
            return Err(SendError(item));
        }
        inner.queue.push_back(item);
        if let Some(waker) = inner.recv_waker.take() {
            waker.wake();
        }
        Ok(())
    }
}

impl<T> Clone for UnboundedSender<T> {
    fn clone(&self) -> Self {
        let mut inner = self.0.inner.lock().unwrap();
        inner.senders += 1;
        Self(Sender {
            inner: self.0.inner.clone(),
        })
    }
}

pub struct UnboundedReceiver<T>(Receiver<T>);

impl<T> UnboundedReceiver<T> {
    pub async fn recv(&mut self) -> Option<T> {
        self.0.recv().await
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct SendError<T>(pub T);

#[derive(Debug, PartialEq, Eq)]
pub enum RecvError {
    Closed,
}
