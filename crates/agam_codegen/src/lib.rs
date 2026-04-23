//! # agam_codegen
//!
//! Code generation backend for the Agam language.
//!
//! Currently implements a **C transpiler** — MIR → C source code.
//! This allows compilation via any C compiler (gcc, clang, MSVC) and
//! gives immediate cross-platform native binary output.
//!
//! Future backends: LLVM IR, WASM, custom bytecode.

pub mod c_emitter;
pub mod gpu_emitter;
pub mod llvm_emitter;
