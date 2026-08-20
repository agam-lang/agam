//! Asynchronous Timers and Deadline Management.

use std::future::Future as StdFuture;
use std::pin::Pin;
use std::task::{Context as StdContext, Poll as StdPoll};
use std::time::{Duration, Instant};

/// An asynchronous sleep future that completes after a specified duration.
pub struct Sleep {
    deadline: Instant,
}

/// Create a future that suspends until the specified duration has elapsed.
pub fn sleep(duration: Duration) -> Sleep {
    Sleep {
        deadline: Instant::now() + duration,
    }
}

impl StdFuture for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut StdContext<'_>) -> StdPoll<Self::Output> {
        let now = Instant::now();
        if now >= self.deadline {
            StdPoll::Ready(())
        } else {
            let remaining = self.deadline - now;
            let waker = cx.waker().clone();
            std::thread::spawn(move || {
                std::thread::sleep(remaining);
                waker.wake();
            });
            StdPoll::Pending
        }
    }
}

/// Error returned when an asynchronous operation exceeds its time limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Elapsed;

impl std::fmt::Display for Elapsed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "operation timed out")
    }
}

impl std::error::Error for Elapsed {}

/// Wraps a future with a timeout duration.
pub async fn timeout<F: StdFuture>(duration: Duration, future: F) -> Result<F::Output, Elapsed> {
    struct TimeoutFut<F> {
        pin: Pin<Box<F>>,
        deadline: Instant,
    }

    impl<F: StdFuture> StdFuture for TimeoutFut<F> {
        type Output = Result<F::Output, Elapsed>;

        fn poll(self: Pin<&mut Self>, cx: &mut StdContext<'_>) -> StdPoll<Self::Output> {
            let this = unsafe { self.get_unchecked_mut() };
            match this.pin.as_mut().poll(cx) {
                StdPoll::Ready(output) => StdPoll::Ready(Ok(output)),
                StdPoll::Pending => {
                    let now = Instant::now();
                    if now >= this.deadline {
                        StdPoll::Ready(Err(Elapsed))
                    } else {
                        let remaining = this.deadline - now;
                        let waker = cx.waker().clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(remaining);
                            waker.wake();
                        });
                        StdPoll::Pending
                    }
                }
            }
        }
    }

    TimeoutFut {
        pin: Box::pin(future),
        deadline: Instant::now() + duration,
    }
    .await
}

/// A recurring interval timer producing ticks at a specified duration cadence.
pub struct Interval {
    period: Duration,
    next_tick: Instant,
}

pub fn interval(period: Duration) -> Interval {
    Interval {
        period,
        next_tick: Instant::now() + period,
    }
}

impl Interval {
    pub async fn tick(&mut self) -> Instant {
        let now = Instant::now();
        if self.next_tick > now {
            sleep(self.next_tick - now).await;
        }
        let fired_at = self.next_tick;
        self.next_tick += self.period;
        fired_at
    }
}
