# Compiler Pipeline Overview

Current state of the Agam compiler pipeline as of May 2026.

## Pipeline Stages

```
Source (.agam)
    │
    ▼
agam_lexer          → Token stream
    │
    ▼
agam_parser         → AST (agam_ast crate)
    │
    ▼
agam_sema           → Semantic analysis, type checking, HIR lowering
    │
    ▼
agam_hir            → HIR (High-level IR, typed, effect-annotated)
    │
    ▼
agam_mir::lower     → MIR (Mid-level IR, SSA-like, lowered ops)
    │
    ▼
agam_codegen ──────────────────────────────────────────┐
    ├→ C backend (agam_c_emitter)     → .c → clang/gcc │
    ├→ LLVM backend (agam_llvm)       → LLVM IR → opt  │
    │       └→ NVPTX path (@gpu)      → PTX / CUDA     │
    │       └→ SPIR-V path (Phase 29) → OpenCL/Vulkan  │
    └→ JIT backend (agam_jit)         → in-process exec │
```

## Crate Responsibilities

| Crate | Role |
|-------|------|
| `agam_lexer` | Tokenizer, keyword table, span tracking |
| `agam_parser` | Recursive-descent Pratt parser, AST construction |
| `agam_ast` | AST node definitions, visitor traits |
| `agam_sema` | Name resolution, type checking, effects analysis, HIR lowering, target validation |
| `agam_hir` | HIR node definitions: typed expressions, effect nodes, target profiles |
| `agam_mir` | MIR node definitions, `agam_mir::lower` (HIR→MIR), optimization passes |
| `agam_codegen` | Backend dispatch to C/LLVM/JIT |
| `agam_runtime` | Runtime support: OS sandbox, hardware detection, async event loop (future) |
| `agam_std` | Standard library: I/O, effects, collections |
| `agam_interface` | Shared parse/sema/HIR/MIR orchestration + `WarmState` |
| `agam_driver` | CLI entry point: `agamc` subcommand dispatch |
| `agam_lsp` | Language server (incremental compilation via daemon) |
| `agam_pkg` | Package resolution, manifest, lockfile, registry |
| `agam_ffi` | Python/LangChain/LlamaIndex bindings |

## What Is Working End-to-End (May 2026)

- ✅ Parse → HIR → MIR → C backend (basic programs, effects, `@gpu` kernels)
- ✅ Parse → HIR → MIR → LLVM backend (basic programs, `@gpu` NVPTX)
- ✅ JIT execution (`agamc run` via in-process LLVM)
- ✅ Algebraic effects: `perform`, `handle`, `effect` (Phase 20 complete)
- ✅ OS sandbox enforcement (Phase 21 complete)
- ✅ Omni-targeting directives: `@target.iot`, `@target.hpc`, `@target.enterprise` (Phase 22)
- ✅ GPU kernel pipeline: `@gpu(...)`, `GpuKernelLaunch` MIR op, NVPTX emitter (Phase 23 partial)
- ✅ Incremental daemon, warm state reuse, parallel compilation (Phase 15F)

## Known Compiler Gaps (see also `mir-todo-gaps.md`)

- ⬜ No generics in the type system — affects all collection types
- ⬜ No sum types / enums in sema/MIR (HIR nodes exist; lowering incomplete)
- ⬜ No pattern matching lowering through MIR to codegen
- ⬜ Struct literal lowering in `agam_mir::lower` has `todo!()` paths
- ⬜ Enum variant and `match` lowering in `agam_mir::lower` has `todo!()` paths
- ⬜ Backend enum-layout and enum codegen incomplete
- ⬜ No memory model enforcement (ARC, ownership, or borrow checker)

## Key Architectural Facts

- **`agam_interface`** is the stable public API over parse/sema/HIR/MIR — CLI, LSP, and daemon all go through this
- **`WarmState`** caches per-file AST/HIR/MIR across incremental compilation cycles
- The **C backend** is the "always works" fallback; LLVM backend is production-primary
- The **JIT backend** is used by `agamc repl` and `agamc exec` for interactive/headless execution
- Effects are currently **dispatched via external functions** (C and LLVM backends) — not yet through a proper runtime handler dispatch table
