//! # agam_runtime
//!
//! Lightweight runtime for the Agam language.
//!
//! Provides:
//! - **ARC** — atomic reference counting for the default memory mode.
//! - **Scheduler** (future) — M:N green thread scheduler.
//! - **Actors** (future) — message-passing actor system.

pub mod arc;
