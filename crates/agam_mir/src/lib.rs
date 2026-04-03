//! # agam_mir
//!
//! Mid-level Intermediate Representation with SSA form and Control Flow Graph.
//!
//! The MIR is a low-level, register-based IR suitable for optimization
//! and code generation. It uses:
//! - **Basic blocks** with explicit terminators (branch, return, jump).
//! - **SSA values** (each value assigned exactly once).
//! - **CFG** (control flow graph) for optimization passes.

pub mod analysis;
pub mod ir;
pub mod lower;
pub mod opt;
