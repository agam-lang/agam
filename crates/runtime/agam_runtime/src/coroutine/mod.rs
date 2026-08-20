//! Production-Grade Stackless Coroutine & Asynchronous Concurrency Runtime.
//!
//! Provides:
//! - M:N Work-stealing coroutine scheduler (`Runtime`, `spawn`, `block_on`, `spawn_blocking`)
//! - Stackless Coroutine & Resumption Engine (`Coroutine`, `CoroutineState`, `Generator`)
//! - Structured Concurrency Nurseries (`TaskGroup`)
//! - Async Synchronization (`AsyncMutex`, `AsyncSemaphore`, `AsyncBarrier`)
//! - Lock-free async channels (`channel`, `unbounded_channel`, `oneshot`)
//! - Async Timers & Deadlines (`sleep`, `timeout`, `interval`)
//! - Async Combinators (`select`, `join`, `yield_now`)

pub mod channel;
pub mod combinators;
pub mod nursery;
pub mod scheduler;
pub mod state_machine;
pub mod sync;
pub mod task;
pub mod timer;

pub use channel::{
    Receiver, RecvError, SendError, Sender, UnboundedReceiver, UnboundedSender, channel, oneshot,
    unbounded_channel,
};
pub use combinators::{Either, join, select};
pub use nursery::TaskGroup;
pub use scheduler::{Runtime, RuntimeBuilder, YieldNow, yield_now};
pub use state_machine::{Coroutine, CoroutineState, Generator, StateTag};
pub use sync::{AsyncBarrier, AsyncMutex, AsyncMutexGuard, AsyncSemaphore, SemaphorePermit};
pub use task::{JoinHandle, Poll, SchedulableTask, TaskError, TaskId};
pub use timer::{Elapsed, Interval, Sleep, interval, sleep, timeout};

#[cfg(test)]
mod tests {
    use super::*;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;

    // ══════════════════════════════════════════════════════════════════════
    // 1. Stackless Coroutine State Machine Tests
    // ══════════════════════════════════════════════════════════════════════

    struct CountdownCoroutine {
        current: u32,
    }

    impl Coroutine<()> for CountdownCoroutine {
        type Yield = u32;
        type Return = &'static str;

        fn resume(
            mut self: Pin<&mut Self>,
            _input: (),
        ) -> CoroutineState<Self::Yield, Self::Return> {
            if self.current > 0 {
                let val = self.current;
                self.current -= 1;
                CoroutineState::Yielded(val)
            } else {
                CoroutineState::Complete("liftoff!")
            }
        }
    }

    #[test]
    fn test_coroutine_step_resumption() {
        let mut coro = CountdownCoroutine { current: 3 };
        let mut pin = Pin::new(&mut coro);

        assert_eq!(pin.as_mut().resume(()), CoroutineState::Yielded(3));
        assert_eq!(pin.as_mut().resume(()), CoroutineState::Yielded(2));
        assert_eq!(pin.as_mut().resume(()), CoroutineState::Yielded(1));
        assert_eq!(
            pin.as_mut().resume(()),
            CoroutineState::Complete("liftoff!")
        );
    }

    // ══════════════════════════════════════════════════════════════════════
    // 2. M:N Scheduler & Task Spawning Tests
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_runtime_spawn_and_join() {
        let rt = RuntimeBuilder::new().worker_threads(2).build();

        let handle = rt.spawn(async {
            let mut sum = 0;
            for i in 1..=100 {
                sum += i;
            }
            sum
        });

        let res = rt.block_on(async { handle.await.unwrap() });
        assert_eq!(res, 5050);
    }

    #[test]
    fn test_runtime_spawn_blocking() {
        let rt = RuntimeBuilder::new().worker_threads(2).build();

        let handle = rt.spawn_blocking(|| {
            std::thread::sleep(Duration::from_millis(5));
            42 * 2
        });

        let res = rt.block_on(async { handle.await.unwrap() });
        assert_eq!(res, 84);
    }

    // ══════════════════════════════════════════════════════════════════════
    // 3. Async Synchronization & Channel Tests
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_async_mutex_contention() {
        let rt = RuntimeBuilder::new().worker_threads(4).build();
        let counter = Arc::new(AsyncMutex::new(0));

        let mut handles = Vec::new();
        for _ in 0..10 {
            let c = counter.clone();
            handles.push(rt.spawn(async move {
                let mut guard = c.lock().await;
                *guard += 1;
            }));
        }

        rt.block_on(async {
            for h in handles {
                h.await.unwrap();
            }
            let val = *counter.lock().await;
            assert_eq!(val, 10);
        });
    }

    #[test]
    fn test_async_mpsc_channel() {
        let rt = RuntimeBuilder::new().worker_threads(2).build();
        let (tx, mut rx) = channel(5);

        rt.spawn(async move {
            for i in 1..=5 {
                tx.send(i * 10).await.unwrap();
            }
        });

        let sum = rt.block_on(async {
            let mut total = 0;
            while let Some(val) = rx.recv().await {
                total += val;
                if val == 50 {
                    break;
                }
            }
            total
        });

        assert_eq!(sum, 10 + 20 + 30 + 40 + 50);
    }

    // ══════════════════════════════════════════════════════════════════════
    // 4. Structured Concurrency Nursery Tests
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_structured_nursery_task_group() {
        let rt = RuntimeBuilder::new().worker_threads(2).build();
        let accumulator = Arc::new(AtomicU32::new(0));

        rt.block_on(async {
            let nursery = TaskGroup::new(&rt);

            for _ in 0..5 {
                let acc = accumulator.clone();
                nursery.spawn(async move {
                    acc.fetch_add(10, Ordering::Relaxed);
                });
            }

            nursery.wait_all().await.unwrap();
            assert_eq!(accumulator.load(Ordering::Relaxed), 50);
        });
    }

    // ══════════════════════════════════════════════════════════════════════
    // 5. Async Combinator & Timeout Tests
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_async_select_and_join() {
        let rt = RuntimeBuilder::new().worker_threads(2).build();

        rt.block_on(async {
            let f1 = async { 10 };
            let f2 = async { 20 };
            let (r1, r2) = join(f1, f2).await;
            assert_eq!(r1, 10);
            assert_eq!(r2, 20);

            let s1 = async { "winner" };
            let s2 = async {
                sleep(Duration::from_millis(50)).await;
                "loser"
            };
            let sel = select(s1, s2).await;
            assert_eq!(sel, Either::Left("winner"));
        });
    }
}
