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
pub mod effects;
pub mod hwinfo;
pub mod sandbox;
pub mod simd;
