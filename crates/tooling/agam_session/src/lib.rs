//! Compiler session management and worker pipeline orchestration.

#![deny(clippy::unwrap_used)]

pub mod config;
pub mod pipeline;

pub use config::{BackendKind, OptLevel, SessionConfig};
pub use pipeline::{CompiledArtifact, CompilerSession};
