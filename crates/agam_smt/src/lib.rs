//! Refinement type verification via SMT solving.
//!
//! Exposes an SMT-LIB2 interface to verify compile-time constraints
//! like division by zero, out-of-bounds array access, and integer overflow.

pub mod solver;
pub mod verify;
