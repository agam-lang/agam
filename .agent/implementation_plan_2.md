# Agam Compiler — Advanced Features Roadmap (Phases 6–15)

Building on the completed Phases 1–5 (Lexer → Parser → AST → Semantic Analysis → IR/Codegen), this plan maps **10 breakthrough features** into concrete implementation phases across existing crates.

---

## Phase 6: Hybrid Memory Ownership Model (ARC + Strict Lifetimes)

> Unify Python's simplicity with Rust's safety via a dual-mode memory manager.

### Crates: `agam_runtime`, `agam_sema` (ownership.rs, lifetime.rs)

#### [MODIFY] [ownership.rs](file:///c:/Projects/agam/crates/agam_sema/src/ownership.rs)
- Add `MemoryMode` enum: `ARC` (default) | `Strict` (opt-in)
- In ARC mode: insert reference count ops, no borrow checker errors
- In Strict mode: enforce existing borrow/lifetime rules (zero-cost)

#### [MODIFY] [lifetime.rs](file:///c:/Projects/agam/crates/agam_sema/src/lifetime.rs)
- Skip lifetime checks when `MemoryMode::ARC` is active
- Only enforce region-based lifetimes inside `strict { }` blocks

#### [NEW] [arc.rs](file:///c:/Projects/agam/crates/agam_runtime/src/arc.rs)
- `AgamArc<T>` — atomic reference-counted pointer with cycle detection
- `AgamWeak<T>` — weak reference for breaking cycles
- Integration with `agam_codegen` to emit [arc_retain](file:///c:/Projects/agam/crates/agam_sema/src/ownership.rs#259-263)/`arc_release` calls

#### [MODIFY] [c_emitter.rs](file:///c:/Projects/agam/crates/agam_codegen/src/c_emitter.rs)
- Emit `agam_arc_new()`, [agam_arc_retain()](file:///c:/Projects/agam/crates/agam_runtime/src/arc.rs#159-167), `agam_arc_release()` for ARC mode
- Emit raw stack allocations for Strict mode

---

## Phase 7: First-Class Differentiable Programming (Native AI)

> The compiler natively understands mathematical derivatives — no PyTorch needed.

### Crates: `agam_lexer`, `agam_ast`, `agam_hir`, `agam_mir`

#### [MODIFY] [token.rs](file:///c:/Projects/agam/crates/agam_lexer/src/token.rs)
- Add `TokenKind::Grad` keyword for automatic differentiation

#### [MODIFY] [expr.rs](file:///c:/Projects/agam/crates/agam_ast/src/expr.rs)
- Add `ExprKind::Grad { func: Box<Expr>, wrt: Ident }` — ∂f/∂x
- Add `ExprKind::Backward { expr: Box<Expr> }` — reverse-mode AD

#### [NEW] [autodiff.rs](file:///c:/Projects/agam/crates/agam_hir/src/autodiff.rs)
- **Forward-mode AD**: dual numbers propagation through the HIR
- **Reverse-mode AD**: automatic backward pass generation using chain rule: `∂z/∂x = ∂z/∂y · ∂y/∂x`
- Instruction-level gradient tape recording in MIR

#### [NEW] [tensor.rs](file:///c:/Projects/agam/crates/agam_std/src/tensor.rs)
- `Tensor<T, Shape>` — N-dimensional array with shape-typed dimensions
- BLAS-backed matrix operations (matmul, conv2d, softmax)
- Auto-batching and broadcasting rules

---

## Phase 8: Hardware-Aware Execution and Benchmarking

> Agam natively understands CPU/GPU/NPU topology for extreme perf tuning.

### Crates: `agam_macro`, `agam_sema`, `agam_codegen`, `agam_runtime`

#### [NEW] [hardware.rs](file:///c:/Projects/agam/crates/agam_macro/src/hardware.rs)
- `#[align(L1_Cache)]` — enforce memory layout to cache line boundaries
- `#[dispatch(SIMD)]` — emit vectorized instructions (SSE/AVX/NEON)
- `#[dispatch(GPU)]` — offload computation to GPU compute shaders
- `#[prefetch]` — hint cache prefetch on data-heavy loops

#### [MODIFY] [checker.rs](file:///c:/Projects/agam/crates/agam_sema/src/checker.rs)
- Validate hardware annotations (e.g., error if `#[align(L1_Cache)]` on a struct exceeding 64 bytes)
- Memory bandwidth formula validation: `B = f × W × n`

#### [NEW] [simd_emitter.rs](file:///c:/Projects/agam/crates/agam_codegen/src/simd_emitter.rs)
- Emit SIMD intrinsics for `#[dispatch(SIMD)]` annotated loops
- Platform detection: SSE4.2 / AVX2 / AVX-512 / ARM NEON

#### [NEW] [hwinfo.rs](file:///c:/Projects/agam/crates/agam_runtime/src/hwinfo.rs)
- Runtime CPU topology detection (cache sizes, core count, NUMA nodes)
- `@platform` annotation for conditional compilation per architecture

---

## Phase 9: Algebraic Effects System

> Replace hardcoded async/await, exceptions, and generators with composable effects.

### Crates: `agam_lexer`, `agam_ast`, `agam_sema`, `agam_hir`

#### [MODIFY] [token.rs](file:///c:/Projects/agam/crates/agam_lexer/src/token.rs)
- Add `TokenKind::Effect`, `TokenKind::Handle`, `TokenKind::Resume`

#### [MODIFY] [decl.rs](file:///c:/Projects/agam/crates/agam_ast/src/decl.rs)
- `DeclKind::Effect { name, ops: Vec<EffectOp> }` — define effect interfaces
- `DeclKind::Handler { effect, handlers: Vec<HandlerArm> }` — effect handlers

#### [NEW] [effects.rs](file:///c:/Projects/agam/crates/agam_sema/src/effects.rs)
- Effect type tracking through function signatures
- Effect polymorphism resolution
- "Function coloring" elimination — async/sync unification

#### [MODIFY] [lower.rs](file:///c:/Projects/agam/crates/agam_hir/src/lower.rs)
- Desugar effect [perform](file:///c:/Projects/agam/crates/agam_sema/src/effects.rs#171-181) into delimited continuations
- Transform handlers into continuation-passing style (CPS)

---

## Phase 10: Refinement Types with SMT Solving

> Prove mathematical constraints at compile time — no division by zero, ever.

### Crates: `agam_ast`, `agam_sema`, new `agam_smt`

#### [MODIFY] [types.rs](file:///c:/Projects/agam/crates/agam_ast/src/types.rs)
- `TypeExprKind::Refined { base: Box<TypeExpr>, predicate: Box<Expr> }`
- Example: `{v: Int | v != 0}` — guarantees non-zero at compile time

#### [NEW] `crates/agam_smt/` — SMT solver integration
- **`solver.rs`** — interface to Z3/CVC5 solver (or embedded micro-solver)
- **`constraints.rs`** — translate refinement predicates into SMT-LIB2 assertions
- **`verify.rs`** — check satisfiability of type constraints at compile time
- Prove: array bounds, division safety, integer overflow, null safety

#### [MODIFY] [checker.rs](file:///c:/Projects/agam/crates/agam_sema/src/checker.rs)
- When a refinement type is encountered, extract predicate and send to SMT solver
- Cache verification results per function for incremental compilation
---

## Phase 10B: Advanced ML Syntax & Execution Pipeline

> Bridge the syntactic gap to execute our Data Science benchmarks natively without Rust wrappers.

### Crates: `agam_parser`, `agam_driver`, `agam_codegen`

#### [MODIFY] [expr.rs](file:///c:/Projects/agam/crates/agam_parser/src/expr.rs)
- Implement closure parsing: `|x, y| { x + y }` or `|x| x.sin()`
- Implement struct initialization literal parsing: `DataFrame { id: 1 }`
- Implement array literal sizing bounds: `[1.0, 2.0]`

#### [MODIFY] [check.rs](file:///c:/Projects/agam/crates/agam_sema/src/checker.rs)
- Type check closure captures and environmental bindings
- Ensure inferred struct literal types match definitions in `agam_std`

#### [MODIFY] [main.rs](file:///c:/Projects/agam/crates/agam_driver/src/main.rs)
- Wire up `agamc build` and `agamc run`
- Hook the pipeline: `Lex → Parse → Type Check → HIR → MIR → agm_codegen (C) → invoke gcc/clang → Run`
- Execute `.agam` files natively end-to-end to capture true benchmark timings without Rust simulation.

---

## Phase 10C: Compilation Optimization Passes (The Speedup Phase)

> Close the 63x gap with Rust/C++ by aggressively optimizing the Intermediate Representation before handing it to the C compiler.

### Crates: `agam_mir`, `agam_codegen`

Status update:
- Completed checkpoint 10C.1: added a MIR optimization pipeline entrypoint, implemented `constant_fold.rs`, and wired the optimizer into `agamc build` before C emission.
- Completed checkpoint 10C.2: added conservative small-function MIR inlining, dead-code elimination for unreachable blocks and unused locals, and a MIR `Copy` op to preserve SSA results after inlining.
- Completed checkpoint 10C.3: upgraded the C emitter to infer and emit concrete C types (`agam_int`, `agam_float`, `agam_bool`, `agam_str`) per value/local/function, fixed non-main return truncation, and added basic string concatenation support in the runtime prelude.

#### [NEW] `crates/agam_mir/src/opt/` — IR Optimization Passes
- **`constant_fold.rs`**: Evaluate `let x = 5 * 10;` into `let x = 50;` at compile time.
- **`inline.rs`**: Replace calls to small functions with their actual basic blocks to avoid call overhead.
- **`dce.rs`**: Dead Code Elimination to remove unreachable blocks and unused `Alloca` variables.

#### [MODIFY] [c_emitter.rs](file:///c:/Projects/agam/crates/agam_codegen/src/c_emitter.rs)
- **Monomorphization**: Emit specialized types (`int32_t`, `double`) instead of a generic `agam_int`. This allows the CPU to perfectly pack data into L1 cache and drastically speeds up execution.

---

## Phase 10D: Native ML & Dataframe Hardware Acceleration

> Make the `bench_ml.agam` script actually run on the C-backend without throwing "missing stub" errors, and benchmark it against Pandas.

Status update:
- Completed the C-backend bridge for native ML/dataframe benchmarking by teaching `agam_codegen` about opaque `Tensor` / `DataFrame` handle types and known builtin signatures.
- Added real C runtime-prelude implementations for `adam`, dataframe build/filter/sort/group-by/mean/free, and tensor fill/dense/conv/checksum/free paths.
- Added a backend-compatible `bench_ml.agam` benchmark that avoids the unsupported lambda/module-call surface and now builds and runs end-to-end through `agamc build`.
- Verified the native benchmark from WSL. The Pandas comparison could not be executed in this environment because `numpy` and `pandas` are not installed in WSL.

### Crates: `agam_std`, `agam_codegen`

#### [MODIFY] [c_emitter.rs](file:///c:/Projects/agam/crates/agam_codegen/src/c_emitter.rs)
- Implement actual C-runtime bodies for `agam_filter`, `agam_group_by`, and `agam_sort`.
- Translate Agam's `Tensor` and `DataFrame` AST structs into highly optimized packed C-structs.
- Implement the forward and backward passes of `agam_dense_layer` and `agam_conv2d` using simple native Matrix multiplication loops (which will be auto-vectorized by our Phase 8 hardware tags).


---

## Priority 1: Compiler Optimization & Native Execution

### Phase 11: Intermediate Representation (MIR) Optimization
> *The Concept*: Before converting to machine code, the compiler simplifies the abstract syntax tree into an intermediate format.
> *The Coding Improvement*: Implement compiler passes in the `agam_mir` crate for Dead Code Elimination (DCE), Constant Folding, and Loop Unrolling to drastically reduce instructions. 

Status update:
- The MIR optimizer now lives in `agam_mir::opt` and is fully wired into `agamc build` before C emission.
- Constant folding, local constant propagation, dead-code elimination, and conservative leaf-function inlining were already completed during Phase 10C and now serve as the base of Phase 11.
- Added `loop_unroll.rs` for conservative fixed-trip counted-loop unrolling on the current MIR shape (`preheader -> cond -> body -> cond`) when the trip count is statically provable and small.
- The optimization pipeline now runs `inline -> constant_fold -> loop_unroll -> constant_fold -> dce` to a fixed point.

#### Crates: `agam_mir`
- **`opt/dce.rs`**: Traverse CFG and remove unreachable basic blocks / unused allocas.
- **`opt/constant_fold.rs`**: Evaluate literal math loops at compile time.
- **`opt/loop_unroll.rs`**: Unroll fixed-bound loops. $T_{unrolled} \approx \frac{T_{original}}{k} + C$.

### Phase 12: Hardware Intrinsic Dispatch (SIMD)
> *The Concept*: Single Instruction, Multiple Data (SIMD) allows processing multiple tensor elements in one CPU clock cycle.

Status update:
- Extended `agam_runtime::simd` from auto-vectorizable scalar loops into runtime-dispatched intrinsic kernels for x86 (`SSE2`, `AVX`, `AVX-512`) and AArch64 (`NEON`) on the core numeric operations: add, sub, mul, scale, dot, FMA, and the tiled matmul micro-kernel.
- Kept scalar fallbacks and runtime hardware selection via `hwinfo().simd.best_tier()` so the same codepath remains portable across machines.
- Wired `agam_std::tensor` hot paths (`add`, `sub`, `mul`, `scale`, `dot`, `matmul`) into `SimdOps`, and routed `agam_std::ndarray::norm` plus ML similarity/distance helpers through the SIMD runtime as well.
- Verified the runtime and stdlib SIMD path with the existing test suites after adding the new dispatch and tensor integration.

#### Crates: `agam_runtime`, `agam_codegen`
- **`simd.rs`**: Expand Runtime to map the compiler's `#[dispatch(SIMD)]` tags directly to AVX-512 or ARM NEON intrinsics.
- Ensures the processor mathematically vectorizes arrays natively rather than relying on a C compiler's secondary translation.

### Phase 13: Cache Alignment & Memory Locality
> *The Concept*: Unaligned memory causes cache misses, freezing the processor.

Status update:
- Refactored `agam_runtime::arc` away from two disjoint `Box` allocations into a single contiguous `ArcInner<T>` allocation containing both the value and the ARC header.
- Wired the allocator to honor runtime `AlignmentHint`s, with `AgamArc::new()` defaulting to cache-line alignment and `AgamArc::new_aligned()` supporting `CacheLine`, `L1Cache`, `SimdWidth`, or custom alignments.
- Added tests proving that ARC-managed values land on the requested alignment boundary, including the 64-byte cache-line default and SIMD-width alignment.
- This gives the runtime allocator the concrete alignment/padding hook needed for the `#[align(L1_Cache)]` family of annotations.

#### Crates: `agam_runtime`
- **`arc.rs`**: Implement the `#[align(L1_Cache)]` macro logic inside the memory allocator.
- Pad memory allocations to exactly 64-byte boundaries, ensuring continuous tensor blocks slot perfectly into the L1 cache.

### Phase 14: Direct LLVM IR or JIT Backend
> *The Concept*: Bypassing intermediate text files (like C code) to emit machine code directly into memory.

#### Crates: `agam_codegen`, `agam_jit`
- **`llvm_emitter.rs`**: Transition the C-transpiler to emit LLVM IR (`.ll`) via the `inkwell` crate.
- **`agam_jit`**: Build a Cranelift-based JIT compiler to evaluate code on-the-fly at runtime for interactive environments.

---

## Priority 2: Developer Experience & Ecosystem

### Phase 15: Developer Tooling (LSP & Formatter)
#### Crates: `agam_lsp`, `agam_fmt`, `agam_test`
- **`agam_lsp`**: Implement Language Server Protocol (TCP/Stdio) providing real-time autocomplete, hover type-checking, and diagnostics.
- **`agam_fmt`**: A strict AST auto-formatter ensuring consistent syntax rules.
- **`agam_test`**: A built-in unit testing module utilizing the compiler driver.

### Phase 16: Interactive REPL & Sandboxed Execution
#### Crates: `agam_notebook`, `agam_jit`
- **REPL Interface**: Allows rapid data-science experimentation with line-by-line tensor evaluations without full build steps.
- **Headless API**: Build a server endpoint where an LLM sends an Agam string, and the JIT compiler evaluates it and formats output strictly as JSON.

### Phase 17: Ecosystem & Dependency Management
#### Crates: `agam_pkg`, `agam_std`
- **`agam_pkg`**: Content-Addressable Package Manager. Resolve dependencies by hashing ASTs using BLAKE3. Perfect reproducibility across 100-server clusters.
- **`agam_std`**: Expand standard library for native networking and File I/O directly via algebraic effects.

---

## Priority 3: Agentic LLM Integration & Ecosystem

To make it easy for the AI community to adopt Agam, we must build official plugins for popular agent frameworks.

### Phase 18: Build an Agam Execution Tool (The Interface)
- Create a secure sandboxed tool where the LLM can send a string of Agam code.
- **Workflow**: LLM outputs Agam script -> Tool writes to `temp.agam` -> Tool calls `agamc run temp.agam` -> Tool returns `stdout/stderr` back to LLM.

### Phase 19: LangChain & LlamaIndex Wrappers
- **`AgamREPLTool`**: Write a custom Python class for LangChain that acts as a drop-in replacement for their default `PythonREPLTool`. This allows developers to swap Python for Agam with a single line of code.
- **`agam_ffi`**: Implement FFI so Python frameworks can quickly execute compiled native Agam libraries.

### Phase 20: Tree-Sitter & Prompt Engineering
- **Tree-Sitter Grammar**: Write formal Tree-sitter strict grammar rules. Advanced LLMs use this to constrain token generation, preventing accidental Python hallucinations into Agam scripts.
- **System Instructions & Few-Shot**: Enforce context injection: "You are an expert in Agam. Do not use Python".
- **Context Serializer**: A tool to automatically grab 5-10 perfect examples of Agam's `tensor` and `grad` syntax and inject them into the LLM prompt.

### Phase 21: Model Fine-Tuning (The Long-Term Solution)
- **Dataset Creation**: Collect thousands of `.agam` scripts and their expected ML outputs.
- **Model Training**: Fine-tune an open-source model (LLaMA-3 or Mistral) on this dataset so it natively "thinks" in Agam without needing heavy prompt engineering.

---

## Priority 4: Omni-Targeting & Bare-Metal Hardware (The Omni-Language)

Agam must be truly ubiquitous—scaling elegantly from bare-metal IoT microcontrollers to heavy enterprise AI clusters, bypassing all OS bloat where needed.

### Phase 22: Omni-Targeting Directives
#### Crates: `agam_sema`, `agam_codegen`
- **`@target.iot`**: The compiler strips the `agam_std` library entirely (`no_std`), strips the ARC memory manager, and refuses heap allocations. Produces sub-10KB statically linked binaries suitable for ESP32 and Arduino.
- **`@target.enterprise`**: Enables runtime Garbage Collection (if chosen over ARC), massive thread pooling, and enforces the Capability-Based Security Sandbox via the `agam_runtime` OS boundary.
- **`@target.hpc`**: High-Performance Computing mode—instructs `agam_codegen` and `agam_mir` to maximize auto-vectorization over memory efficiency.

### Phase 23: GPU & NPU Integration (CUDA / ROCm)
#### Crates: `agam_codegen`, `agam_macro`
- **`@gpu` Macro**: `#[gpu(threads=512)] fn matrix_multiply(...)` allowing native kernel compilation directly in Agam (no C++ PyTorch wrappers needed).
- **CUDA/ROCm backends**: Expand `llvm_emitter.rs` to target `nvptx64` (Nvidia) and `amdgcn` (AMD ROCm) natively.
- **MLIR Dialects for NPUs**: Convert our AST Matrix operations into MLIR graphs suitable for execution on Apple Neural Engine and Qualcomm Hexagon TPUs.

### Phase 24: Advanced DSA & Scientific Computing
#### Crates: `agam_std`
- **Sparse Mechanics**: Natively optimized types for Sparse Matrices (crucial for massive NLP models and GNNs).
- **`agam_std::collections`**: A famously fast standard library module featuring Native Lock-Free Queues, B-Trees, Graph adjacency formats, and vectorized HashMaps that natively rival C++'s `absl::flat_hash_map`.
- **`agam_std::math`**: Native Fast Fourier Transforms (FFT) and highly optimized mathematical solvers.

### Phase 25: Machine-Code Control & Optimizations
#### Crates: `agam_parser`, `agam_driver`
- **Inline Assembly**: Adding `asm! { "mov eax, 1" }` blocks for exact CPU instruction access for kernel and IoT developers.
- **Profile-Guided Optimization (PGO)**: Add `--pgo-generate` and `--pgo-use` to `agam_driver`. Agam will profile a program's execution to analyze common `if/else` branches, then mathematically recompile the exact binary to minimize branch-prediction misses.

---

## Priority 5: Future Frontier Implementations

### Phase 26: Quantum & ZKP Primitives
- **`Qubit` Types**: Implement native Qubit types and gates.
- **`#[zkp]` macros**: Compile mathematical logic straight into zero-knowledge proofs (zk-SNARKs).
