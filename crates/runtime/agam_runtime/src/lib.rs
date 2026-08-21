//! # agam_runtime
//!
//! Lightweight runtime for the Agam language.
//!
//! Provides:
//! - **ARC** — atomic reference counting for the default memory mode.
//! - **HWInfo** — CPU topology detection (cores, caches, SIMD features).
//! - **SIMD** — portable SIMD operations with auto-dispatch.
//! - **Sandbox** — OS-level execution isolation (Job Objects, prctl).
//! - **Effects** — runtime effect handler dispatch table.
//! - **Scheduler** (future) — M:N green thread scheduler.
//! - **Actors** (future) — message-passing actor system.

pub mod arc;
pub mod cache;
pub mod contract;
pub mod coroutine;
pub mod crypto;
pub mod effects;
pub mod hwinfo;
pub mod sandbox;
pub mod security;
pub mod simd;

pub use crypto::{Sha256, chacha20_xor, hmac_sha256, sha256_digest};
pub use hwinfo::{
    GpuTelemetry, HardwareInfo, MemoryTopology, NpuTelemetry, PerfTelemetry, SimdCapabilities,
    SimdTier, hwinfo,
};
pub use security::{Secret, SecureRandom, constant_time_eq, zeroize};

pub use coroutine::{
    AsyncBarrier, AsyncCondvar, AsyncMutex, AsyncPipe, AsyncRead, AsyncRwLock, AsyncSemaphore,
    AsyncWrite, Coroutine, CoroutineState, Generator, JoinHandle, Poll, Runtime, RuntimeBuilder,
    RuntimeMetrics, TaskGroup, TaskId, channel, join, oneshot, select, sleep, timeout,
    unbounded_channel, yield_now,
};
