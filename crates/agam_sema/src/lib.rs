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

pub mod symbol;
pub mod types;
pub mod scope;
pub mod resolver;
