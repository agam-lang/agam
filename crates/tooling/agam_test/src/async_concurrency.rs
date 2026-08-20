//! Comprehensive Async Concurrency & Coroutine Integration Testing Suite.
//!
//! Verifies:
//! 1. High-throughput M:N work-stealing scheduler execution under multicore load.
//! 2. Race condition prevention with AsyncMutex, AsyncRwLock, and AsyncCondvar.
//! 3. Structured concurrency nurseries with cascade cancellation on timeout.
//! 4. Non-blocking asynchronous I/O streaming via AsyncPipe.
//! 5. Micro-benchmark measuring coroutine spawn/join latency.

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    use agam_runtime::coroutine::{
        AsyncCondvar, AsyncMutex, AsyncPipe, AsyncRwLock, RuntimeBuilder, TaskGroup, sleep,
    };

    #[test]
    fn test_async_high_concurrency_task_flood() {
        let rt = RuntimeBuilder::new().worker_threads(4).build();
        let counter = Arc::new(AtomicUsize::new(0));

        let start = Instant::now();
        let num_tasks = 2_000;

        let mut handles = Vec::with_capacity(num_tasks);
        for _ in 0..num_tasks {
            let c = counter.clone();
            handles.push(rt.spawn(async move {
                c.fetch_add(1, Ordering::Relaxed);
            }));
        }

        rt.block_on(async {
            for h in handles {
                h.await.unwrap();
            }
        });

        let elapsed = start.elapsed();
        assert_eq!(counter.load(Ordering::Relaxed), num_tasks);
        let tasks_per_sec = num_tasks as f64 / elapsed.as_secs_f64().max(0.0001);
        assert!(
            tasks_per_sec > 5_000.0,
            "Task spawning throughput was {tasks_per_sec:.2} tasks/sec"
        );
    }

    #[test]
    fn test_async_rwlock_exclusive_mutation_integrity() {
        let rt = RuntimeBuilder::new().worker_threads(4).build();
        let rw = Arc::new(AsyncRwLock::new(Vec::<usize>::new()));

        let mut handles = Vec::new();
        // Spawn 10 concurrent writers
        for i in 0..10 {
            let rw_clone = rw.clone();
            handles.push(rt.spawn(async move {
                let mut guard = rw_clone.write().await;
                guard.push(i);
            }));
        }

        rt.block_on(async {
            for h in handles {
                h.await.unwrap();
            }
            let guard = rw.read().await;
            assert_eq!(guard.len(), 10);
        });
    }

    #[test]
    fn test_async_condvar_producer_consumer_queue() {
        let rt = RuntimeBuilder::new().worker_threads(2).build();
        let pair = Arc::new((AsyncMutex::new(Vec::new()), AsyncCondvar::new()));

        let pair_producer = pair.clone();
        rt.spawn(async move {
            for i in 1..=5 {
                sleep(Duration::from_millis(2)).await;
                let (lock, cvar) = &*pair_producer;
                let mut q = lock.lock().await;
                q.push(i);
                cvar.notify_one();
            }
        });

        rt.block_on(async {
            let (lock, cvar) = &*pair;
            let mut collected = Vec::new();
            while collected.len() < 5 {
                let mut q = lock.lock().await;
                while q.is_empty() {
                    q = cvar.wait(q).await;
                }
                while let Some(val) = q.pop() {
                    collected.push(val);
                }
            }
            assert_eq!(collected.len(), 5);
        });
    }

    #[test]
    fn test_async_pipe_large_payload_streaming() {
        let rt = RuntimeBuilder::new().worker_threads(2).build();
        let pipe = Arc::new(AsyncPipe::new(1024));

        let p_writer = pipe.clone();
        let payload = vec![0xABu8; 16384]; // 16 KB payload

        let payload_clone = payload.clone();
        rt.spawn(async move {
            p_writer.write_all(&payload_clone).await.unwrap();
            p_writer.close();
        });

        let received = rt.block_on(async {
            let mut buf = [0u8; 512];
            let mut all_bytes = Vec::with_capacity(16384);
            loop {
                let n = pipe.read(&mut buf).await.unwrap();
                if n == 0 {
                    break;
                }
                all_bytes.extend_from_slice(&buf[..n]);
            }
            all_bytes
        });

        assert_eq!(received.len(), 16384);
        assert_eq!(received, payload);
    }

    #[test]
    fn test_async_nursery_cascade_cancellation() {
        let rt = RuntimeBuilder::new().worker_threads(2).build();
        let counter = Arc::new(AtomicUsize::new(0));

        let c1 = counter.clone();
        let c2 = counter.clone();

        rt.block_on(async {
            let nursery = TaskGroup::new(&rt);

            nursery.spawn(async move {
                c1.fetch_add(1, Ordering::Relaxed);
                sleep(Duration::from_millis(100)).await;
            });

            nursery.spawn(async move {
                c2.fetch_add(1, Ordering::Relaxed);
                sleep(Duration::from_millis(100)).await;
            });

            // Nursery cancels all child tasks if timeout is exceeded
            let res = nursery.wait_with_timeout(Duration::from_millis(5)).await;
            assert!(res.is_err(), "must report timeout error");
        });
    }
}
