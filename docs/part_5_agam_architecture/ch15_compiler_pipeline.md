# Chapter 15: End-to-End Agam Compiler Pipeline Walkthrough

> **System Scope**: Full Agam Compiler Lifecycle & Driver Architecture  
> **Compiler Module Focus**: [`agam_driver`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/tooling/agam_driver), [`agam_pkg`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/tooling/agam_pkg)

---

## 15.1 Complete Source-to-Binary Execution Flow

The `agamc` CLI orchestrates the full compilation lifecycle through a carefully layered pipeline of independently testable transformations. Each stage consumes the output of the previous stage and produces a well-defined intermediate artifact:

```text
 ┌──────────────────────────────────────────────────────────────────────────┐
 │                    AGAM COMPILATION PIPELINE                            │
 ├──────────────────────────────────────────────────────────────────────────┤
 │                                                                          │
 │  Source Code (.agam)                                                     │
 │        │                                                                 │
 │        ▼                                                                 │
 │  ┌─────────────┐     Token Stream                                       │
 │  │ agam_lexer   │────────────────┐                                       │
 │  └─────────────┘                 │                                       │
 │                                  ▼                                       │
 │                          ┌──────────────┐     Untyped AST                │
 │                          │ agam_parser   │─────────────┐                 │
 │                          └──────────────┘              │                 │
 │                                                        ▼                 │
 │                                               ┌────────────┐            │
 │                                               │ agam_sema   │           │
 │                                               │ Type Check  │           │
 │                                               │ Scope Resolve│          │
 │                                               └──────┬─────┘           │
 │                                                       │ Typed AST       │
 │                                                       ▼                 │
 │                                               ┌────────────┐            │
 │                                               │ agam_hir    │           │
 │                                               │ Desugar     │           │
 │                                               │ Pattern Dec │           │
 │                                               └──────┬─────┘           │
 │                                                       │ HIR             │
 │                                                       ▼                 │
 │                                               ┌────────────┐            │
 │                                               │ agam_mir    │           │
 │                                               │ SSA / CFG   │           │
 │                                               │ Opt Passes   │          │
 │                                               └──────┬─────┘           │
 │                                                       │ Optimized MIR   │
 │                          ┌────────────────────────────┼──────────┐      │
 │                          ▼                            ▼          ▼      │
 │                  ┌──────────────┐          ┌──────────┐  ┌──────────┐  │
 │                  │ agam_codegen  │          │ agam_jit  │  │ SPIR-V   │  │
 │                  │ LLVM / C11   │          │ Cranelift │  │ NVPTX    │  │
 │                  └──────┬───────┘          └────┬─────┘  └────┬─────┘  │
 │                         ▼                       ▼              ▼        │
 │                  Native Binary           JIT Execution   GPU Kernel     │
 │                  (.exe / .elf)            (In-Process)   (.spv / .ptx)  │
 └──────────────────────────────────────────────────────────────────────────┘
```

---

## 15.2 Phase-by-Phase Timing & Data Flow

Each phase has distinct performance characteristics and output artifacts:

| # | Phase | Crate | Input | Output | Typical Time |
| :---: | :--- | :--- | :--- | :--- | :--- |
| 1 | **Lexing** | `agam_lexer` | UTF-8 source bytes | `Vec<Token>` with `Span` positions | ~2 μs/KB |
| 2 | **Parsing** | `agam_parser` | Token stream | `ast::Module` (untyped AST tree) | ~5 μs/KB |
| 3 | **Semantic Analysis** | `agam_sema` | Untyped AST | Typed AST + scope graph + diagnostics | ~15 μs/KB |
| 4 | **HIR Lowering** | `agam_hir` | Typed AST | Desugared HIR (decision trees, closures) | ~8 μs/KB |
| 5 | **MIR Generation** | `agam_mir` | HIR | SSA Basic Blocks + CFG | ~10 μs/KB |
| 6 | **MIR Optimization** | `agam_mir::opt` | Unoptimized MIR | Optimized MIR (SCCP, DCE, inlining) | ~20 μs/KB |
| 7 | **Code Generation** | `agam_codegen` | Optimized MIR | LLVM IR / C11 / SPIR-V / NVPTX | ~30 μs/KB |
| 8 | **Linking** | LLVM `lld` / system | Object files | Native executable | ~50 ms |

**Target throughput**: > 500,000 lines/sec for lexing, > 100,000 lines/sec for full pipeline to JIT execution.

---

## 15.3 Crate Dependency Architecture

The compiler's 27 crates are organized in strict dependency layers. No crate may depend on a crate in a higher layer:

```text
Layer 0 (Foundation):
  agam_errors ─── agam_interface

Layer 1 (Core Frontend):
  agam_lexer ──► agam_parser ──► agam_ast
       │              │              │
       └──────────────┴──────────────┘
                      │ all depend on agam_errors

Layer 2 (Middle-End):
  agam_sema ──► agam_hir ──► agam_mir
       │              │            │
       └──── depend on Layer 1 ────┘

Layer 3 (Backends):
  agam_codegen ──► agam_jit
       │                │
       └── depend on Layer 2 + agam_runtime

Layer 4 (Runtime):
  agam_runtime ──► agam_std
       │                │
       └── standalone, minimal deps

Layer 5 (Tooling):
  agam_driver ──► agam_pkg ──► agam_lsp ──► agam_fmt
  agam_doc ──► agam_lint ──► agam_test ──► agam_profile
  agam_debug
       │
       └── depend on all lower layers

Layer 6 (Experiments):
  agam_ffi ──► agam_game ──► agam_macro ──► agam_notebook
  agam_smt ──► agam_ui
```

**Key invariant:** No circular dependencies exist. The dependency graph is a strict DAG verified by `cargo check --workspace`.

---

## 15.4 Driver Coordination & Command CLI (`agam_driver`)

The CLI entrypoint (`agamc`) provides a unified interface for all developer workflows. Each command maps to a specific pipeline depth:

| CLI Command | Pipeline Depth | Action | Primary Crate Targets |
| :--- | :---: | :--- | :--- |
| `agamc build` | Full | Complete compilation to native binary | `agam_driver` → `agam_codegen` → `lld` |
| `agamc run` | Full + Exec | Build and execute target binary | `agam_driver` → `agam_runtime` |
| `agamc check` | Layers 1–2 | Fast type checking and diagnostics | `agam_lexer` → `agam_sema` |
| `agamc repl` | Full (JIT) | Interactive REPL with JIT execution | `agam_driver` → `agam_jit` |
| `agamc dev` | Incremental | Warm-daemon incremental build loop | `agam_driver` → `DaemonSession` |
| `agamc exec` | Full + Sandbox | Headless agent execution with resource limits | `agam_driver` → `agam_notebook` |
| `agamc doctor` | Diagnostic | Verify host LLVM and C toolchain | `agam_driver` → `agam_runtime` |
| `agamc test` | Full + Test | Compile and run test suite | `agam_driver` → `agam_test` |
| `agamc fmt` | Parse only | Format source code | `agam_driver` → `agam_fmt` |
| `agamc lint` | Layers 1–3 | Static lint analysis | `agam_driver` → `agam_lint` |
| `agamc doc` | Layers 1–2 | Generate HTML documentation | `agam_driver` → `agam_doc` |
| `agamc new` | Scaffold | Create new project from template | `agam_driver` → `agam_pkg` |
| `agamc add` | Manifest | Add dependency to `agam.toml` | `agam_driver` → `agam_pkg` |
| `agamc publish` | Full + Registry | Publish package to registry | `agam_driver` → `agam_pkg` |

---

## 15.5 Error Propagation Strategy

Errors and diagnostics flow through a unified `agam_errors` reporting system used by every pipeline phase:

```text
Phase Error → agam_errors::Diagnostic {
    severity: Error | Warning | Note | Help,
    message: String,
    span: Span { source_id, start, end },
    labels: Vec<Label>,       // Source code annotations
    notes: Vec<String>,       // Additional context
    fix_suggestions: Vec<Fix> // Machine-applicable fixes
}
```

**Error recovery philosophy:**
- The **lexer** recovers from invalid characters by emitting an `ErrorToken` and advancing past the invalid byte sequence.
- The **parser** uses **synchronization tokens** (`;`, `}`, `fn`, `struct`) to recover from syntax errors and continue parsing subsequent declarations.
- The **semantic analyzer** collects *all* type errors in a single pass rather than aborting on the first error, enabling batch error display.
- Errors are rendered using the **Nyāya 4-Part Proof** diagnostic format (Thesis, Reason, Example, Application) for pedagogically superior error messages.

---

## 15.6 Multi-Target Code Generation Dispatch

After MIR optimization, the driver dispatches to the appropriate backend based on the target profile and `@gpu`/`@target` annotations:

```text
Optimized MIR
      │
      ├─── @target.native (default) ──► LLVM IR Emitter ──► LLVM Opt ──► lld ──► .exe/.elf
      │
      ├─── @target.c ─────────────────► C11 Emitter ──► cc/gcc/clang ──► .exe/.elf
      │
      ├─── @target.wasm ──────────────► WASM Emitter ──► .wasm (WASI 0.2)
      │
      ├─── @gpu (NVIDIA) ─────────────► NVPTX Adapter ──► .ptx
      │
      ├─── @gpu (Vendor-Neutral) ─────► SPIR-V Emitter ──► .spv (Vulkan/OpenCL)
      │
      ├─── @gpu (Apple) ──────────────► Metal Adapter ──► .metallib
      │
      └─── JIT (agamc repl / exec) ──► Cranelift / LLVM ORC JIT ──► In-Process
```

**Fat-Binary bundling:** When compiling for multiple targets simultaneously, `agam_codegen::link_opt::FatBinaryBundle` packages multiple architecture-specific binaries into a single distribution artifact with runtime dispatch based on `cpuid` / device enumeration.

---

## 15.7 Incremental Compilation Boundaries

The incremental daemon (`DaemonSession`) caches pipeline artifacts at three well-defined boundaries:

| Cache Level | Artifact Cached | Invalidation Trigger |
| :--- | :--- | :--- |
| **L1 — Token Cache** | Serialized token streams per file | Source file content hash change |
| **L2 — AST/HIR Cache** | Typed AST and desugared HIR per module | Any file in the module's dependency cone changes |
| **L3 — MIR Cache** | Optimized MIR per function | Function body or any called function signature changes |

**Fingerprinting:** Each source file is fingerprinted using a fast content hash (`xxHash64`). The `WorkspaceSnapshot` stores `HashMap<PathBuf, u64>` mapping file paths to fingerprints. On each recompilation request, `WorkspaceSnapshotDiff` compares current fingerprints against the cached snapshot to identify the minimal set of invalidated modules.
