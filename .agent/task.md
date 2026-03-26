# Agam Language — Master Task List


## Phase 0: Architectural Design & Module Outline
- [x] Create initial Architectural Design Document (ADD)
- [x] Define the first 50 compiler bootstrap phases (Milestones A–F)
- [x] Address user questions (NumPy, notebooks, UI, games, kernel, threading, envs)
- [x] Add phases 51–75 (Milestones G–J: FFI, threading, packages, notebook)
- [x] Add expert-recommended features (error handling, contracts, hot reload, LSP, security, DSLs)
- [x] Add phases 76–100 (Milestones K–O: toolchain, testing, cross-compilation, self-hosting)
- [ ] Get user approval on Phase 0 deliverables

## Phase 1: Project Infrastructure (Milestone A, Phases 1–5)
- [x] Initialize Rust workspace with 25 crate stubs
- [x] Implement `agam_errors` crate (span, diagnostic, report)
- [x] Implement `agam_driver` CLI skeleton (6 subcommands)
- [x] Set up test harness (40 tests passing)
- [ ] Write initial grammar spec (EBNF)

## Phase 2: Lexer (Milestone B, Phases 6–15)
- [x] Define token types (TokenKind enum — all keywords, operators, literals)
- [x] Implement character stream / cursor (UTF-8 aware)
- [x] Implement full tokenizer (all operators, numbers, strings, comments, keywords)
- [x] 28 lexer unit tests passing (more needed for 50+ target)
- [x] Mode detection (`@lang.base`, `@lang.base.dynamic`, vs `@lang.advance`)
- [x] Indentation-to-brace synthesis for base mode

## Phase 3: AST & Parser (Milestones C–D, Phases 16–32)
- [x] Define AST node types (expr, stmt, decl, types, pattern, visitor, pretty)
- [x] Integrate dual typing ([var](file:///c:/Projects/agam/crates/agam_std/src/ndarray.rs#163-168), `dyn`, `static` mode, `TypeMode`)
- [x] Implement recursive-descent / Pratt parser (15 precedence levels)
- [x] Wire parser into `agamc check` CLI
- [x] End-to-end verification of `hello_base`, `hello_base_dynamic`, and `hello_advance` examples

## Phase 4: Semantic Analysis (Milestone E, Phases 33–42)
- [x] **Phase 33: Symbol table & Scoped Name Resolution**
- [x] Phase 34: Type Representation (Primitives, Generics, Type Variables)
- [x] Phase 35: Type Inference Engine (Hindley-Milner Algorithm W)
- [x] Phase 36: Type Unification & Constraint Solving
- [x] Phase 37: Trait Resolution & Method Dispatch
- [x] Phase 38: Ownership Analysis (Move Semantics, Borrow Tracking)
- [x] Phase 39: Lifetime Analysis (Region Inference, Elision Rules)
- [x] Phase 40: Pattern Exhaustiveness Checking (`match`)
- [x] Phase 41: Const Evaluation / Comptime Execution
- [x] Phase 42: Semantic Analysis Test Suite (100+ cases)

## Phase 5: IR & Code Generation (Milestone F, Phases 43–50)
- [x] HIR/MIR definition and lowering
- [x] C transpiler codegen backend
- [x] End-to-end: `return 42` → C source output

## Phase 6: Hybrid Memory (ARC + Strict Lifetimes)
- [x] `MemoryMode` enum (ARC default, Strict opt-in) in [ownership.rs](file:///c:/Projects/agam/crates/agam_sema/src/ownership.rs)
- [x] `AgamArc<T>` runtime reference counting in `agam_runtime/arc.rs`
- [x] `strict { }` block enforcement in `lifetime.rs`
- [x] C emitter: [arc_retain](file:///c:/Projects/agam/crates/agam_sema/src/ownership.rs#259-263)/`arc_release` calls

## Phase 7: First-Class Differentiable Programming
- [x] [grad](file:///c:/Projects/agam/bench_ml.py#7-12) keyword in lexer + `ExprKind::Grad` in AST
- [x] Forward-mode AD (dual numbers) in `agam_hir/autodiff.rs`
- [x] Reverse-mode AD (backward pass via chain rule) in MIR
- [x] `Tensor<T, Shape>` in `agam_std/tensor.rs`

## Phase 7B: Math & Science Stdlib (Hardware-Optimized)
- [x] [math.rs](file:///c:/Projects/agam/crates/agam_std/src/math.rs) — integration (Simpson, Gauss), FFT, root-finding
- [x] [linalg.rs](file:///c:/Projects/agam/crates/agam_std/src/linalg.rs) — LU/QR decomposition, eigenvalues, determinant, inverse
- [x] [stats.rs](file:///c:/Projects/agam/crates/agam_std/src/stats.rs) — distributions, PRNG, hypothesis testing
- [x] [complex.rs](file:///c:/Projects/agam/crates/agam_std/src/complex.rs) — complex numbers, quaternions
- [x] [units.rs](file:///c:/Projects/agam/crates/agam_std/src/units.rs) — compile-time dimensional analysis (SI units)
- [x] [precision.rs](file:///c:/Projects/agam/crates/agam_std/src/precision.rs) — BigInt, interval arithmetic
- [x] [numerical.rs](file:///c:/Projects/agam/crates/agam_std/src/numerical.rs) — Newton-Raphson, gradient descent, ODE solvers
- [x] [dataframe.rs](file:///c:/Projects/agam/crates/agam_std/src/dataframe.rs) — typed columnar DataFrame (filter, sort, group_by, describe)
- [x] [ndarray.rs](file:///c:/Projects/agam/crates/agam_std/src/ndarray.rs) — NumPy-like ops (arange, linspace, reshape, argmax, stack, outer, eye)
- [x] [ml.rs](file:///c:/Projects/agam/bench_ml.rs) — loss functions, activations (GELU/Swish), dense layer, batch norm, KNN, metrics

## Phase 8: Hardware-Aware Execution
- [x] `#[align(L1_Cache)]`, `#[dispatch(SIMD/GPU)]` macros
- [x] SIMD intrinsics emitter for SSE/AVX/NEON
- [x] Runtime CPU topology detection in `agam_runtime/hwinfo.rs`
- [x] Cache-line validation in [checker.rs](file:///c:/Projects/agam/crates/agam_sema/src/checker.rs)

## Phase 9: Algebraic Effects System
- [x] [effect](file:///c:/Projects/agam/crates/agam_sema/src/effects.rs#83-87), [handle](file:///c:/Projects/agam/crates/agam_sema/src/effects.rs#88-92), [resume](file:///c:/Projects/agam/crates/agam_sema/src/effects.rs#182-190) keywords
- [x] `DeclKind::Effect` + `DeclKind::Handler` in AST
- [x] Effect type tracking + polymorphism in `agam_sema/effects.rs`
- [x] CPS transformation in HIR lowering

## Phase 10: Refinement Types with SMT Solving
- [x] `TypeExprKind::Refined { base, predicate }` in AST
- [x] `agam_smt` crate: SMT-LIB2 solver interface
- [x] Compile-time proofs: div-by-zero, array bounds, overflow
- [x] Verification result caching for incremental compilation

## Phase 10B: Advanced ML Syntax & Execution Pipeline
- [x] Parse closure expressions: `|x, y| { ... }` natively in `agam_parser/parser.rs`
- [x] Parse struct instantiations: `dataframe.DataFrame { x: 1 }`
- [x] Implement array literals with variable lengths: `[1, 2, 3]`
- [x] Wire `agamc build` and `agamc run` to fully output native executables via `agam_codegen`
- [x] Create comprehensive 4-way benchmark suite (Agam vs Python vs Rust vs C++)

## Phase 10C: Compilation Optimization Passes (The Speedup Phase)
- [x] Constant Folding & Propagation (pre-calculate mathematical constants at compile time)
- [ ] Monomorphization (Specialize generic numbers into specific `int32`/`float64` types instead of boxed `agam_int`)
- [ ] Function Inlining (replace small method calls with their actual body in MIR)
- [ ] Dead Code Elimination (remove unused variables and branches before sending to GCC)

## Phase 10D: Native ML & Dataframe Acceleration API
- [ ] Implement C-compatible structs for [DataFrame](file:///c:/Projects/agam/crates/agam_std/src/dataframe.rs#72-75) and `NdArray`
- [ ] Write native C-backend implementations for `agam_filter`, `agam_sort`, and `agam_group_by`
- [ ] Connect `agam_adam`, `agam_dense_layer`, and `agam_conv2d` stubs to actual optimized C matrix loops
- [ ] Execute `bench_ml.agam` (Dataframes + Neural Networks) natively and benchmark vs Pandas/PyTorch

## Priority 1: Compiler Optimization & Native Execution

### Phase 11: Intermediate Representation (MIR) Optimization
- [ ] Implement `agam_opt` module for MIR optimization passes
- [ ] Implement Constant Folding
- [ ] Implement Dead Code Elimination (DCE)
- [ ] Implement Loop Unrolling ($T_{unrolled} \approx \frac{T_{original}}{k} + C$)

### Phase 12: Hardware Intrinsic Dispatch (SIMD)
- [ ] Expand `agam_runtime::simd` module
- [ ] Map `#[dispatch(SIMD)]` annotations directly to AVX-512 and ARM NEON intrinsics
- [ ] Natively vectorize tensor arrays without relying on secondary C translation

### Phase 13: Cache Alignment & Memory Locality
- [ ] Implement `#[align(L1_Cache)]` macro logic
- [ ] Update memory allocator (`agam_runtime::arc`) to pad allocations to 64-byte boundaries
- [ ] Ensure continuous tensor blocks slot perfectly into CPU L1 cache

### Phase 14: Direct LLVM IR or JIT Backend
- [ ] Transition `agam_codegen` away from C-transpilation to direct LLVM IR emission
- [ ] Alternatively, build `agam_jit` using Cranelift to compile Agam to machine code on-the-fly

---

## Priority 2: Developer Experience & Ecosystem

### Phase 15: Developer Tooling (LSP & Formatter)
- [ ] Build `agam_lsp` (Language Server Protocol) for real-time autocomplete, type-checking on hover, and error highlighting in VS Code / Neovim.
- [ ] Implement `agam_fmt` for consistent syntax styling
- [ ] Implement `agam_test` for reliable built-in unit testing

### Phase 16: Interactive REPL & Sandboxed Execution
- [ ] Build `agam_notebook` / `agam_jit` to create a Read-Eval-Print Loop (REPL) for instant tensor evaluation without full build steps.
- [ ] Build a Headless API Sandbox server endpoint to accept Agam strings, JIT evaluate, and return strict JSON `stdout/stderr`.

### Phase 17: Dependency Management & Standard Library
- [ ] Build `agam_pkg` a content-addressable manager using BLAKE3 AST hashing to ensure perfect reproducibility across compute clusters.
- [ ] Expand `agam_std` to natively handle networking and file I/O safely via Algebraic Effects rather than messy async/await.

---

## Priority 3: Agentic LLM Integration & Ecosystem

### Phase 18: Build an Agam Execution Tool (The LLM Interface)
- [ ] Create a secure sandboxed tool where an LLM inputs a string of Agam code.
- [ ] Workflow: Tool writes temp [.agam](file:///c:/Projects/agam/bench.agam) file -> invokes `agamc run` -> feeds output/errors back to LLM.

### Phase 19: LangChain & LlamaIndex Wrappers
- [ ] Build `AgamREPLTool` native Python class.
- [ ] Publish as a drop-in replacement for `PythonREPLTool` for LangChain and LlamaIndex to allow rapid AI community adoption.
- [ ] Build `agam_ffi` to allow existing Python frameworks to seamlessly call compiled native Agam.

### Phase 20: Tree-Sitter & Prompt Engineering
- [ ] Write a formal Tree-Sitter grammar for Agam to constrain LLM token generation (preventing Python hallucinations).
- [ ] Context Injection system prompts ("You are an expert in Agam... Do not use Python").
- [ ] Build a Few-Shot Context Serializer to inject 5-10 perfect ML syntax examples automatically.

### Phase 21: Model Fine-Tuning (The Long-Term Solution)
- [ ] Dataset Creation: Generate/collect thousands of [.agam](file:///c:/Projects/agam/bench.agam) scripts and expected outputs.
- [ ] Model Training: Fine-tune an open-source model (Llama-3, Mistral) to natively "think" in Agam syntax.

---

## Priority 4: Omni-Targeting & Bare-Metal Hardware (The Omni-Language)

### Phase 22: Omni-Targeting Directives
- [ ] Implement `@target.iot` to strip `agam_std` and ARC memory for `<10KB` raw static binaries.
- [ ] Implement `@target.enterprise` enabling GC, heavy threading, and capability-sandbox locking.

### Phase 23: GPU & NPU Integration (CUDA / ROCm)
- [ ] Transition `agam_codegen` to emit LLVM NVPTX (CUDA) and AMDGPU (ROCm) assembly.
- [ ] Build `@gpu(threads=512)` kernel macros for native ML execution without C++ wrappers.
- [ ] Implement MLIR dialects for Neural Processing Units (NPU).

### Phase 24: Advanced DSA & Scientific Computing
- [ ] Implement Sparse Matrix hardware-types for massive NLP / GNN models.
- [ ] Build native Fast Fourier Transforms (FFT) and Signal Processing inside `agam_std::math`.
- [ ] Build a notorious lock-free, highly vectorized `agam_std::collections` (B-Trees, Graphs, Maps).

### Phase 25: Machine-Code Control & Optimizations
- [ ] Implement Inline Assembly parsing: `asm! { "mov eax, 1" }` for direct silicon control.
- [ ] Add Profile-Guided Optimization (PGO) loop inside the compiler driver.

---

## Priority 5: Future Frontiers

### Phase 26: Quantum & ZKP Primitives
- [ ] Native `Qubit` types and Quantum gates (H, X, Y, Z, CNOT).
- [ ] `#[zkp]` macros to compile mathematical logic straight into zero-knowledge proofs (zk-SNARKs).
