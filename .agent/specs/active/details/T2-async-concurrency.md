# Phase T2-async-concurrency � Async and Structured Concurrency

**Status:** open
**Tier:** 2 (Runtime and Concurrency)

## Scope

Design and implement Agam's concurrency model with an async runtime, structured concurrency primitives, channels, and synchronization. No modern language can ship without a real concurrency story.

## Design Options

- **async/await** with built-in runtime (like Rust/JS/C#)
- **Green threads** with goroutine-style scheduling (like Go)
- **Actor model** with message passing (like Erlang/Swift)
- **Effect-based concurrency** leveraging Agam's `perform`/`handle` system

## Deliverables

### Async Primitives
- [ ] `async fn` declaration and `await` expression (or chosen equivalent)
- [ ] Future/Promise type representing deferred computation
- [ ] Task spawning: `spawn(async_fn())` or equivalent
- [ ] Task cancellation and timeout

### Structured Concurrency
- [ ] Task groups / nurseries (all child tasks complete before parent)
- [ ] Cancellation propagation through task trees
- [ ] Error propagation from child tasks to parent
- [ ] Scoped concurrency (no leaked tasks)

### Channels
- [ ] Bounded and unbounded channels for inter-task communication
- [ ] `send` / `receive` operations
- [ ] Select/multiplex across multiple channels
- [ ] Channel closing semantics

### Synchronization
- [ ] Mutex, RwLock
- [ ] Semaphore, Barrier
- [ ] Once (one-time initialization)
- [ ] Atomic operations for lock-free data structures

### Runtime
- [ ] Event loop with platform-native I/O multiplexing (epoll/IOCP/kqueue)
- [ ] Work-stealing thread pool for CPU-bound tasks
- [ ] I/O thread pool for blocking operations
- [ ] Timer infrastructure

## Responsible Crates

- `agam_runtime` — async runtime, event loop, thread pool
- `agam_sema` — async type checking, effect interaction
- `agam_hir` / `agam_mir` — async state machine lowering
- `agam_codegen` — coroutine/state machine code generation

## Dependencies

- Phase T2-memory-model (memory model) — concurrency safety depends on memory safety
- Phase T0-type-system (type system) — `Future<T>` needs generics

## Test Strategy

- Concurrency correctness tests (no data races, no deadlocks)
- Structured concurrency tests (task cleanup on cancellation)
- Performance benchmarks vs Go goroutines and Rust tokio
