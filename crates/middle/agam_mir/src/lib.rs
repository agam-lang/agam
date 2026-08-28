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
pub mod dialect;
pub mod eval;
pub mod ir;
pub mod lower;
pub mod monomorphize;
pub mod opt;
pub mod scev;
pub mod verifier;

pub use analysis::{
    AliasOracle, AliasRelation, ControlFlowGraph, DisjointnessProof, DominanceFrontier,
    DominatorTree, LoopForest, NaturalLoop, PointerProvenance, ReversePostOrder,
};
pub use dialect::{
    AsyncDialectOp, BarrierScope, DialectKind, DialectLoweringEngine, GpuDialectOp, MultiLevelOp,
    TensorOp, TensorReduceKind,
};
pub use eval::{ComptimeError, ComptimeInterpreter, ConstValue};
pub use scev::{LoopDescriptor, LoopNest, ScevExpr, ScevSolver, TripCount, lower_to_affine};
pub use verifier::{MirVerificationError, MirVerifier};
