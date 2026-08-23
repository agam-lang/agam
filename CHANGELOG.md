# Changelog

All notable changes to the Agam Programming Language and Compiler Toolchain are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0-alpha.1] — 2026-08-23

### 🚀 Highlights
* **Full Multi-Tier Compiler Pipeline**: End-to-end lexing, Pratt parsing, bidirectional type inference, Sandhi monomorphization, MIR optimization, and multi-backend codegen (LLVM IR, C99, Cranelift JIT, and Universal GPU Emitter).
* **Algebraic Effects & Handlers**: Built-in `perform` and `handle` semantics for deterministic I/O, process management, and cancellation without colored functions.
* **Universal GPU Emitter**: Multi-vendor GPU kernel generation targeting NVPTX, AMDGPU, SPIR-V, and Metal with cooperative matrix multiplication (`OpCooperativeMatrixMulAddKHR`) and tile asynchronous memory (TMA).
* **Dual Memory Model**: Default Automatic Reference Counting (ARC) with compile-time escape analysis, complemented by zero-cost `strict { }` ownership blocks with borrow checking.
* **Formal Verification**: Native SMT contract solver (`agam_smt`) validating function invariants (`requires` / `ensures`) and formal 4-part Nyāya diagnostic proofs (*Pratijñā, Hetu, Udāharaṇa, Nigamana*).
* **Omni-Platform Developer Tooling**: Headless AI execution daemon (`agamc exec`), LSP server with role-label ghost text hinting, package federation (`agamc package`/`publish`), and WebAssembly Playground.

### 📦 Crate Ecosystem
* **Core**: `agam_ast`, `agam_errors`, `agam_lexer`, `agam_parser`, `agam_interface`.
* **Middle**: `agam_sema` (TypeStore arena, ownership, lifetimes, consteval, SMT cache), `agam_hir`, `agam_mir` (CFG, egraphs, monomorphization graph).
* **Backends**: `agam_codegen` (LLVM, C99, GPU, WASM), `agam_jit` (Cranelift JIT execution engine).
* **Runtime**: `agam_runtime` (Chāṇakya OS sandbox, async scheduler, call cache), `agam_std` (math, ML, effects, dataframe, quantum, serial).
* **Tooling**: `agam_driver` (`agamc` CLI), `agam_pkg`, `agam_fmt`, `agam_lint`, `agam_lsp`, `agam_doc`, `agam_debug`, `agam_profile`, `agam_test`.
* **Experiments**: `agam_ffi`, `agam_notebook`, `agam_ui`, `agam_game`, `agam_macro`, `agam_smt`.

### 🛡️ Quality & Verification
* Zero panic paths in runtime libraries (`agam_std::dataframe` panic replaced with `Result<DataFrame, DataFrameError>`).
* 100% test pass rate across 27 workspace crates.
* Zero clippy warnings under `-D warnings`.
