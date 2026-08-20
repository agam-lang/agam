//! Asynchronous Non-Blocking I/O Layer.
//!
//! Provides traits and primitives for asynchronous non-blocking stream reading,
//! writing, and non-blocking in-memory byte pipes with readiness signaling.

use std::collections::VecDeque;
use std::future::Future as StdFuture;
use std::pin::Pin;
use std::sync::Mutex;
use std::task::{Context as StdContext, Poll as StdPoll, Waker};

/// Asynchronous non-blocking read operation trait.
pub trait AsyncRead {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut StdContext<'_>,
        buf: &mut [u8],
    ) -> StdPoll<std::io::Result<usize>>;
}

/// Asynchronous non-blocking write operation trait.
pub trait AsyncWrite {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut StdContext<'_>,
        buf: &[u8],
    ) -> StdPoll<std::io::Result<usize>>;

    fn poll_flush(self: Pin<&mut Self>, cx: &mut StdContext<'_>) -> StdPoll<std::io::Result<()>>;
}

/// In-memory non-blocking byte stream pipe for asynchronous task-to-task streaming.
pub struct AsyncPipe {
    inner: Mutex<PipeState>,
}

struct PipeState {
    buffer: VecDeque<u8>,
    capacity: usize,
    read_waker: Option<Waker>,
    write_waker: Option<Waker>,
    closed: bool,
}

impl AsyncPipe {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(PipeState {
                buffer: VecDeque::with_capacity(capacity.min(65536)),
                capacity: capacity.max(1),
                read_waker: None,
                write_waker: None,
                closed: false,
            }),
        }
    }

    /// Asynchronously read up to `buf.len()` bytes from the pipe.
    pub async fn read(&self, buf: &mut [u8]) -> std::io::Result<usize> {
        struct ReadFut<'a> {
            pipe: &'a AsyncPipe,
            buf: &'a mut [u8],
        }

        impl<'a> StdFuture for ReadFut<'a> {
            type Output = std::io::Result<usize>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut StdContext<'_>) -> StdPoll<Self::Output> {
                let mut state = self.pipe.inner.lock().unwrap();
                if !state.buffer.is_empty() {
                    let to_read = self.buf.len().min(state.buffer.len());
                    for i in 0..to_read {
                        self.buf[i] = state.buffer.pop_front().unwrap();
                    }
                    if let Some(waker) = state.write_waker.take() {
                        waker.wake();
                    }
                    StdPoll::Ready(Ok(to_read))
                } else if state.closed {
                    StdPoll::Ready(Ok(0)) // EOF
                } else {
                    state.read_waker = Some(cx.waker().clone());
                    StdPoll::Pending
                }
            }
        }

        ReadFut { pipe: self, buf }.await
    }

    /// Asynchronously write all bytes from `buf` into the pipe.
    pub async fn write_all(&self, buf: &[u8]) -> std::io::Result<()> {
        let mut offset = 0;
        while offset < buf.len() {
            let written = self.write(&buf[offset..]).await?;
            if written == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    "failed to write whole buffer",
                ));
            }
            offset += written;
        }
        Ok(())
    }

    /// Asynchronously write a chunk of bytes into the pipe.
    pub async fn write(&self, buf: &[u8]) -> std::io::Result<usize> {
        struct WriteFut<'a> {
            pipe: &'a AsyncPipe,
            buf: &'a [u8],
        }

        impl<'a> StdFuture for WriteFut<'a> {
            type Output = std::io::Result<usize>;

            fn poll(self: Pin<&mut Self>, cx: &mut StdContext<'_>) -> StdPoll<Self::Output> {
                let mut state = self.pipe.inner.lock().unwrap();
                if state.closed {
                    return StdPoll::Ready(Err(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        "pipe is closed",
                    )));
                }

                let available = state.capacity.saturating_sub(state.buffer.len());
                if available > 0 {
                    let to_write = self.buf.len().min(available);
                    state.buffer.extend(&self.buf[..to_write]);
                    if let Some(waker) = state.read_waker.take() {
                        waker.wake();
                    }
                    StdPoll::Ready(Ok(to_write))
                } else {
                    state.write_waker = Some(cx.waker().clone());
                    StdPoll::Pending
                }
            }
        }

        WriteFut { pipe: self, buf }.await
    }

    /// Close the pipe signaling EOF to readers.
    pub fn close(&self) {
        let mut state = self.inner.lock().unwrap();
        state.closed = true;
        if let Some(waker) = state.read_waker.take() {
            waker.wake();
        }
        if let Some(waker) = state.write_waker.take() {
            waker.wake();
        }
    }
}
