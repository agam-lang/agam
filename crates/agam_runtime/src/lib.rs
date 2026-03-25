//! # agam_runtime
//!
//! Lightweight runtime for the Agam language.
//!
//! Provides:
//! - **ARC** — atomic reference counting for the default memory mode.
//! - **HWInfo** — CPU topology detection (cores, caches, SIMD features).
//! - **SIMD** — portable SIMD operations with auto-dispatch.
//! - **Scheduler** (future) — M:N green thread scheduler.
//! - **Actors** (future) — message-passing actor system.

pub mod arc;
pub mod hwinfo;
pub mod simd;
