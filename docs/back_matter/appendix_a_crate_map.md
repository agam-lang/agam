# Appendix A: Comprehensive Agam Workspace Crate Map

> **Physical Location**: `agam/crates/{core,middle,backends,runtime,tooling,experiments}`  
> **Total Crates**: 27

---

## Dependency Layer Architecture

```text
Layer 0 ─── Foundation
  agam_errors, agam_interface

Layer 1 ─── Core Frontend
  agam_lexer → agam_parser → agam_ast

Layer 2 ─── Middle-End
  agam_sema → agam_hir → agam_mir

Layer 3 ─── Backends
  agam_codegen, agam_jit

Layer 4 ─── Runtime
  agam_runtime, agam_std

Layer 5 ─── Tooling
  agam_driver, agam_pkg, agam_lsp, agam_fmt,
  agam_doc, agam_lint, agam_test, agam_profile, agam_debug

Layer 6 ─── Experiments
  agam_ffi, agam_game, agam_macro, agam_notebook, agam_smt, agam_ui
```

---

## 1. Core Crates (`crates/core/`) — 5 crates

| # | Crate | Path | Purpose | Key Exports |
| :---: | :--- | :--- | :--- | :--- |
| 1 | [`agam_errors`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/core/agam_errors) | `core/agam_errors` | Centralized diagnostic reporting | `Diagnostic`, `Span`, `SourceId`, `DiagnosticEngine` |
| 2 | [`agam_interface`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/core/agam_interface) | `core/agam_interface` | Shared trait interfaces between crates | `CompilerPass`, `SourceProvider`, `DiagnosticSink` |
| 3 | [`agam_lexer`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/core/agam_lexer) | `core/agam_lexer` | Lexical scanner & tokenizer | `Token`, `TokenKind`, `Lexer`, `Span` |
| 4 | [`agam_parser`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/core/agam_parser) | `core/agam_parser` | Pratt expression & statement parser | `Parser`, `parse_module()`, `parse_expression()` |
| 5 | [`agam_ast`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/core/agam_ast) | `core/agam_ast` | Abstract Syntax Tree node definitions | `Module`, `Stmt`, `Expr`, `TypeExpr`, `AstVisitor` |

---

## 2. Middle-End Crates (`crates/middle/`) — 3 crates

| # | Crate | Path | Purpose | Key Exports |
| :---: | :--- | :--- | :--- | :--- |
| 6 | [`agam_sema`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/middle/agam_sema) | `middle/agam_sema` | Semantic analysis & type checking | `TypeChecker`, `ScopeGraph`, `SymbolTable`, `EffectChecker` |
| 7 | [`agam_hir`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/middle/agam_hir) | `middle/agam_hir` | High-Level IR & desugaring | `HirModule`, `HirExpr`, `PatternDecisionTree`, `ClosureConvert` |
| 8 | [`agam_mir`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/middle/agam_mir) | `middle/agam_mir` | SSA-form MIR & optimization passes | `BasicBlock`, `MirFunction`, `CfgGraph`, `PassManager`, `opt::*` |

---

## 3. Backend Crates (`crates/backends/`) — 2 crates

| # | Crate | Path | Purpose | Key Exports |
| :---: | :--- | :--- | :--- | :--- |
| 9 | [`agam_codegen`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/backends/agam_codegen) | `backends/agam_codegen` | Multi-target code generation | `LlvmEmitter`, `C11Emitter`, `SpirvEmitter`, `NvptxAdapter`, `GpuTuner`, `LayoutOptimizer` |
| 10 | [`agam_jit`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/backends/agam_jit) | `backends/agam_jit` | In-process JIT execution engine | `JitEngine`, `CraneliftBackend`, `LlvmOrcBackend`, `ReplSession` |

---

## 4. Runtime Crates (`crates/runtime/`) — 2 crates

| # | Crate | Path | Purpose | Key Exports |
| :---: | :--- | :--- | :--- | :--- |
| 11 | [`agam_runtime`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/runtime/agam_runtime) | `runtime/agam_runtime` | C ABI bindings & host platform layer | `Allocator`, `HwInfo`, `Sandbox`, `CryptoProvider`, `Coroutine` |
| 12 | [`agam_std`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/runtime/agam_std) | `runtime/agam_std` | Standard library implementations | `FastRingBuffer`, `CompactGraph`, `SparseCSR`, `FFT`, `Tile`, `gpu::*` |

---

## 5. Tooling Crates (`crates/tooling/`) — 9 crates

| # | Crate | Path | Purpose | Key Exports |
| :---: | :--- | :--- | :--- | :--- |
| 13 | [`agam_driver`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/tooling/agam_driver) | `tooling/agam_driver` | Main `agamc` CLI & daemon session | `DriverConfig`, `DaemonSession`, `CompileRequest`, `cli::*` |
| 14 | [`agam_pkg`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/tooling/agam_pkg) | `tooling/agam_pkg` | Package manifest & dependency resolver | `Manifest`, `Lockfile`, `Resolver`, `Registry`, `SemVer` |
| 15 | [`agam_lsp`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/tooling/agam_lsp) | `tooling/agam_lsp` | Language Server Protocol implementation | `LspServer`, `CompletionProvider`, `DiagnosticPublisher`, `HoverProvider` |
| 16 | [`agam_fmt`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/tooling/agam_fmt) | `tooling/agam_fmt` | Source code formatter (CST-preserving) | `Formatter`, `FormatConfig`, `format_file()`, `format_module()` |
| 17 | [`agam_doc`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/tooling/agam_doc) | `tooling/agam_doc` | Documentation generator | `DocBuilder`, `HtmlRenderer`, `CrossRefResolver`, `SearchIndex` |
| 18 | [`agam_lint`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/tooling/agam_lint) | `tooling/agam_lint` | Static analysis lint rules | `LintEngine`, `LintRule`, `lint_correctness::*`, `lint_performance::*` |
| 19 | [`agam_test`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/tooling/agam_test) | `tooling/agam_test` | Test harness & runner | `TestRunner`, `TestSuite`, `Assertion`, `test_macro::*` |
| 20 | [`agam_profile`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/tooling/agam_profile) | `tooling/agam_profile` | Profiling & observability | `TracerProvider`, `MetricExporter`, `BenchHarness`, `Flamegraph` |
| 21 | [`agam_debug`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/tooling/agam_debug) | `tooling/agam_debug` | DWARF debug info & debugger integration | `DwarfEmitter`, `BreakpointManager`, `StackWalker`, `VariableInspector` |

---

## 6. Experiment Crates (`crates/experiments/`) — 6 crates

| # | Crate | Path | Purpose | Key Exports |
| :---: | :--- | :--- | :--- | :--- |
| 22 | [`agam_ffi`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/experiments/agam_ffi) | `experiments/agam_ffi` | C/Python/Rust FFI bindings | `CBindgen`, `PyBufferProtocol`, `WasmExport`, `FfiSafetyChecker` |
| 23 | [`agam_game`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/experiments/agam_game) | `experiments/agam_game` | Game engine integration layer | `SceneGraph`, `RenderPipeline`, `PhysicsWorld`, `ECS` |
| 24 | [`agam_macro`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/experiments/agam_macro) | `experiments/agam_macro` | Procedural macro expansion engine | `MacroExpander`, `DeriveRegistry`, `TokenStreamBuilder` |
| 25 | [`agam_notebook`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/experiments/agam_notebook) | `experiments/agam_notebook` | Headless notebook / `agamc exec` | `NotebookSession`, `CellEvaluator`, `JsonOutputFormatter` |
| 26 | [`agam_smt`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/experiments/agam_smt) | `experiments/agam_smt` | SMT solver integration (Z3/CVC5) | `SmtContext`, `ConstraintBuilder`, `SatResult`, `ModelExtractor` |
| 27 | [`agam_ui`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/experiments/agam_ui) | `experiments/agam_ui` | Declarative UI framework | `Widget`, `LayoutEngine`, `EventLoop`, `StyleSheet`, `Renderer` |

---

## Crate Dependency Summary

| Dependency Layer | Crates | May Depend On |
| :--- | :---: | :--- |
| **Layer 0** — Foundation | 2 | Nothing (leaf crates) |
| **Layer 1** — Core Frontend | 3 | Layer 0 |
| **Layer 2** — Middle-End | 3 | Layers 0–1 |
| **Layer 3** — Backends | 2 | Layers 0–2 |
| **Layer 4** — Runtime | 2 | Layer 0 (minimal deps) |
| **Layer 5** — Tooling | 9 | Layers 0–4 |
| **Layer 6** — Experiments | 6 | Layers 0–5 |

**Strict invariant:** No circular dependencies. The workspace dependency graph is a verified DAG (`cargo check --workspace`).
