# Chapter 19b: Structured Concurrency, Async/Await & Parallel Programming

> **Part VI: The Agam Language Programming Guide**  
> **Compiler Module Focus**: [`agam_runtime::coroutine`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/runtime/agam_runtime), [`agam_std`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/runtime/agam_std)

---

## 19b.1 The Agam Concurrency Model

Agam implements **structured concurrency** — a model where every concurrent task has a well-defined lifetime, a parent scope, and guaranteed cleanup. Unlike Go's goroutines or raw thread spawning, Agam ensures that no task outlives its parent scope and all errors propagate predictably.

```agam
// Structured nursery: all spawned tasks complete before the nursery exits
nursery {
    spawn fetch_user_data(user_id: 42);
    spawn fetch_order_history(user_id: 42);
    spawn fetch_recommendations(user_id: 42);
    // All three tasks run concurrently
    // Nursery waits for ALL to complete before continuing
}
// Guaranteed: all tasks are finished here
println("All data fetched.");
```

---

## 19b.2 Async/Await

For I/O-bound operations, Agam provides `async` functions and `await` expressions:

```agam
async fn fetch_page(url: String) -> Result[String, HttpError] {
    let response = await http.get(url);
    return response.body();
}

async fn main() {
    let page = await fetch_page("https://agam-lang.org");
    match page {
        Result.Ok(body) => println("Page length: " + body.len().to_string()),
        Result.Err(err) => println("Error: " + err.to_string()),
    }
}
```

### Stackless State Machine Compilation

`async fn` compiles into a stackless state machine, similar to Rust's `Future` trait or C#'s async/await:

```text
async fn example() -> Int {
    let a = await step_1();   // Suspend point 1
    let b = await step_2(a);  // Suspend point 2
    return a + b;
}

// Compiled state machine:
enum ExampleState {
    Start,
    WaitingStep1 { },
    WaitingStep2 { a: Int },
    Complete { result: Int },
}
```

Each `await` becomes a state transition. The runtime polls the state machine, advancing it when the awaited value becomes available.

---

## 19b.3 Channels & Message Passing

Agam provides typed channels for safe communication between concurrent tasks:

```agam
// Bounded channel (backpressure when buffer is full)
let (tx, rx) = Channel[Int].bounded(capacity: 100);

nursery {
    // Producer task
    spawn {
        for i in 0..1000 {
            await tx.send(i);  // Suspends if buffer is full
        }
        tx.close();
    }

    // Consumer task
    spawn {
        while let Option.Some(value) = await rx.recv() {
            println("Received: " + value.to_string());
        }
    }
}
```

### Channel Types

| Channel Type | Semantics | Use Case |
| :--- | :--- | :--- |
| `Channel[T].bounded(n)` | Buffered, backpressure at capacity | Producer-consumer pipelines |
| `Channel[T].unbounded()` | Unlimited buffer, never blocks sender | Event streams |
| `Channel[T].rendezvous()` | Zero-buffer, sender waits for receiver | Synchronization points |

---

## 19b.4 Synchronization Primitives

```agam
// Mutex — mutual exclusion for shared state
let counter = Mutex.new(0);

nursery {
    for _ in 0..10 {
        spawn {
            let mut guard = await counter.lock();
            *guard += 1;
            // Mutex automatically released when guard goes out of scope
        }
    }
}
println("Counter: " + counter.into_inner().to_string());  // "10"

// RwLock — multiple readers, single writer
let config = RwLock.new(Config.default());

nursery {
    // Multiple readers can access simultaneously
    spawn { let cfg = await config.read(); process(cfg); }
    spawn { let cfg = await config.read(); validate(cfg); }

    // Writer gets exclusive access
    spawn {
        let mut cfg = await config.write();
        cfg.timeout = 5000;
    }
}
```

---

## 19b.5 Parallel Iterators

For CPU-bound parallelism over collections, Agam provides parallel iterators:

```agam
// Sequential
let results = items.map(fn(item) => expensive_compute(item));

// Parallel — automatically distributes across CPU cores
let results = items.par_map(fn(item) => expensive_compute(item));

// Parallel reduction
let total = numbers.par_reduce(0, fn(acc, x) => acc + x);

// Parallel filter + map
let valid = records
    .par_filter(fn(r) => r.is_valid())
    .par_map(fn(r) => r.transform());
```

### Work-Stealing Scheduler

The M:N coroutine scheduler uses **work-stealing** to balance load across OS threads:

```text
┌──────────────────────────────────────────────────────┐
│                Runtime Thread Pool                     │
│                                                        │
│  Worker 0          Worker 1          Worker 2          │
│  ┌─────────┐      ┌─────────┐      ┌─────────┐      │
│  │ Task A   │      │ Task D   │      │ (idle)  │      │
│  │ Task B   │      │ Task E   │      │         │      │
│  │ Task C   │      │          │      │         │      │
│  └─────────┘      └─────────┘      └────┬────┘      │
│                                          │ steal!     │
│                                          ▼            │
│                                    Steal Task C       │
│                                    from Worker 0      │
└──────────────────────────────────────────────────────┘
```

**Scheduling policy:**
1. Each worker thread has a local double-ended queue (deque) of runnable tasks
2. New tasks are pushed to the local deque
3. A worker pops tasks from its own deque (LIFO for cache locality)
4. When a worker's deque is empty, it **steals** from another worker's deque (FIFO)

---

## 19b.6 Error Handling in Concurrent Code

Agam's structured concurrency model ensures clean error propagation:

```agam
nursery {
    spawn {
        // If this task panics or returns Err...
        let data = await fetch_data()?;
    }
    spawn {
        // ...this task is automatically cancelled
        let report = await generate_report()?;
    }
}
// If ANY spawned task fails, the nursery:
// 1. Cancels all remaining tasks
// 2. Waits for cancellations to complete
// 3. Propagates the first error to the caller
```

This eliminates a common class of concurrency bugs where background tasks continue executing after a sibling has failed, potentially corrupting shared state.
