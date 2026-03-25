//! # agam_sema
//!
//! Semantic analysis for the Agam language.
//!
//! This crate implements the core semantic passes that run after parsing:
//!
//! 1. **Name Resolution** (`resolver`) — walks the AST, declares symbols in the
//!    scope stack, and resolves identifier references to their declarations.
//! 2. **Symbol Table** (`scope`, `symbol`) — manages lexical scopes with
//!    shadowing, redeclaration detection, and dead-code tracking.
//! 3. **Internal Types** (`types`) — resolved type representation with an
//!    interning `TypeStore`, type variables for inference, and well-known
//!    primitive type IDs.
//! 4. **Type Inference** (`infer`) — union-find based constraint solver that
//!    unifies type variables with concrete types.
//! 5. **Type Checking** (`checker`) — walks the AST to generate type constraints
//!    and delegates solving to the inference engine.
//! 6. **Trait Resolution** (`traits`) — trait registry, method dispatch (inherent
//!    before trait), coherence checking, and completeness verification.
//! 7. **Ownership Analysis** (`ownership`) — move tracking, borrow rules,
//!    mutability enforcement, and scope-based drop analysis.

pub mod symbol;
pub mod types;
pub mod scope;
pub mod resolver;
pub mod infer;
pub mod checker;
pub mod traits;
pub mod ownership;
