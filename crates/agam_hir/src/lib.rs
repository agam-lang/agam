//! # agam_hir
//!
//! High-level Intermediate Representation.
//!
//! The HIR is a desugared, type-annotated version of the AST. It:
//! - Removes syntactic sugar (for-in → while + iterator, f-strings → concat).
//! - Attaches resolved type information to every node.
//! - Normalizes control flow (all branches become explicit).
//! - Is the input to MIR lowering.

pub mod nodes;
pub mod lower;
pub mod autodiff;
