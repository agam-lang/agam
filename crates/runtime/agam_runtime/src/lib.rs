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

pub mod actor;
pub mod arc;
pub mod cache;
pub mod capability;
pub mod contract;
pub mod coroutine;
pub mod crypto;
pub mod effects;
pub mod export;
pub mod hwinfo;
pub mod pal;
pub mod pqc;
pub mod sandbox;
pub mod security;
pub mod simd;

pub use actor::{
    Actor, ActorContext, ActorError, ActorId, ActorRef, ActorResult, ActorSystem,
    SupervisionStrategy, SupervisorDirective, SystemMessage,
};

pub use capability::{
    Capability, CapabilitySet, IsolationTier, PermissionDeniedError, global_capabilities,
};
pub use crypto::{Sha256, chacha20_xor, hmac_sha256, sha256_digest};
pub use hwinfo::{
    GpuTelemetry, HardwareInfo, MemoryTopology, NpuTelemetry, PerfTelemetry, SimdCapabilities,
    SimdTier, hwinfo,
};
pub use pal::{
    AllocationFlags, Event, EventDemuxer, EventInterest, MemoryProtection, PageAllocation,
    PalEventError, PalMemoryError, PalNetError, PalTcpListener, PalTcpStream, PalUdpSocket,
    PollTimeout, ShutdownKind, Token, align_to_page_size, system_page_size,
};
pub use pqc::{
    MlDsaKeyPair, MlDsaParameter, MlDsaPublicKey, MlDsaSecretKey, MlKemKeyPair, MlKemParameter,
    MlKemPublicKey, MlKemSecretKey,
};
pub use security::{Secret, SecureRandom, constant_time_eq, zeroize};
pub use simd::{
    AlignedBuffer, AlignmentHint, CpuFeatureDetector, DispatchTarget, SimdError, SimdOps,
    simd_add_f32, simd_add_f64, simd_dot_f32, simd_dot_f64, simd_fma_f32, simd_fma_f64,
    simd_mul_f32, simd_mul_f64,
};

pub use coroutine::{
    AsyncBarrier, AsyncCondvar, AsyncMutex, AsyncPipe, AsyncRead, AsyncRwLock, AsyncSemaphore,
    AsyncWrite, Coroutine, CoroutineState, Generator, JoinHandle, Poll, Runtime, RuntimeBuilder,
    RuntimeMetrics, TaskGroup, TaskId, channel, join, oneshot, select, sleep, timeout,
    unbounded_channel, yield_now,
};
