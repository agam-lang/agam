//! Scalar Evolution (SCEV) and Loop Bound Analysis Subsystem.

pub mod expr;
pub mod lower_to_affine;
pub mod solver;

#[cfg(test)]
mod tests;

pub use expr::ScevExpr;
pub use lower_to_affine::lower_to_affine;
pub use solver::{LoopDescriptor, LoopNest, ScevSolver, TripCount};
