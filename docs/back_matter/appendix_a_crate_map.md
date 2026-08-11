# Appendix A: Comprehensive Agam Crate Reference

> **Physical Location**: `crates/{core,middle,backends,runtime,tooling,experiments}`

---

## Workspace Crate Breakdown

### 1. Core Crates (`crates/core`)
- [`agam_errors`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/core/agam_errors): Centralized diagnostic reporting, `Span`, `SourceId`, color highlighting.
- [`agam_lexer`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/core/agam_lexer): Lexical scanner, token stream generation, UTF-8 position tracking.
- [`agam_parser`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/core/agam_parser): Pratt expression parser and statement parser.
- [`agam_ast`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/core/agam_ast): Abstract Syntax Tree node definitions and visitor traits.

### 2. Middle-End Crates (`crates/middle`)
- [`agam_sema`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/middle/agam_sema): Symbol resolution, nested scope graph, type checker, effect checker.
- [`agam_hir`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/middle/agam_hir): High-Level IR, pattern match desugaring.
- [`agam_mir`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/middle/agam_mir): Medium-Level IR, Basic Blocks, CFG, SSA form, `agam_mir::opt` optimization passes.

### 3. Backend Crates (`crates/backends`)
- [`agam_codegen`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/backends/agam_codegen): LLVM IR lowering, C99 portable fallback code emitter.
- [`agam_jit`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/backends/agam_jit): In-process Cranelift & LLVM ORC JIT execution engine.

### 4. Runtime Crates (`crates/runtime`)
- [`agam_runtime`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/runtime/agam_runtime): C ABI bindings, memory allocator primitives, host detection.
- [`agam_std`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/runtime/agam_std): Standard library runtime definitions.

### 5. Tooling Crates (`crates/tooling`)
- [`agam_driver`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/tooling/agam_driver): Main `agamc` CLI executable driver and `DaemonSession`.
- [`agam_pkg`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/tooling/agam_pkg): `agam.toml` manifest handling, lockfile resolver (`agam.lock`).
- [`agam_lsp`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/tooling/agam_lsp): Language Server Protocol implementation.
- [`agam_fmt`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/tooling/agam_fmt): Source code formatter.
