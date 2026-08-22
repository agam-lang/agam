//! Refinement type verification via SMT solving.
//!
//! Exposes an SMT-LIB2 interface to verify compile-time constraints
//! like division by zero, out-of-bounds array access, and integer overflow.

pub mod contract;
pub mod solver;
pub mod verify;

pub use contract::{Contract, ContractVerifier, VerificationOutcome};
pub use solver::{Constraint, SmtSolver, SolverResult, Z3Solver};
pub use verify::{VerificationCache, VerificationStatus};
