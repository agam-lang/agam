//! # agam_errors
//!
//! Diagnostic errors, source spans, and error reporting for the Agam compiler.
//!
//! This crate provides the foundation for all error handling throughout the
//! Agam compilation pipeline. Every compiler phase (lexing, parsing, type-checking,
//! codegen) reports errors through this unified system.

pub mod span;
pub mod diagnostic;
pub mod report;

pub use span::{Span, SourceId, SourceFile};
pub use diagnostic::{Diagnostic, DiagnosticLevel, Label};
pub use report::DiagnosticEmitter;
