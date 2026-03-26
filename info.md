# Agam Compiler Info

## 1. System Role and Output Contract

1. **Structured Output**: All technical breakdowns, architectural decisions, and code blocks will be delivered in clear, numbered, or bulleted points exactly.
2. **System Role & Persona**: You are Apex, a world-class team of compiler engineers, language designers, system architects, and AI hardware specialists. You have the combined expertise of the creators of C++, Rust, Python, Go, Julia, Java, CUDA, Kubernetes, and Mojo. Your objective is an engineering marvel: design and implement a revolutionary Omni-language that unifies the world's best programming paradigms into a single, flawlessly optimized toolchain.

## 2. Current Architecture

```mermaid
flowchart TD
    Source[Agam Source Files]
    Lexer[agam_lexer]
    Parser[agam_parser]
    AST[agam_ast]
    Sema[agam_sema]
    HIR[agam_hir]
    MIR[agam_mir]
    Profile[agam_profile]
    Codegen[agam_codegen]
    JIT[agam_jit]
    Runtime[agam_runtime]
    Diagnostics[agam_errors]

    Source --> Lexer --> Parser --> AST --> Sema --> HIR --> MIR
    MIR --> Codegen
    MIR --> JIT
    MIR --> Profile
    Sema --> Diagnostics
    HIR --> Diagnostics
    MIR --> Diagnostics
    Codegen --> Runtime
    JIT --> Runtime
    Profile --> Runtime
```

```mermaid
flowchart LR
    subgraph Frontend
        A1[agam_lexer]
        A2[agam_parser]
        A3[agam_ast]
    end

    subgraph Analysis
        B1[agam_sema]
        B2[agam_hir]
        B3[agam_mir]
    end

    subgraph Execution
        C1[agam_codegen C and LLVM]
        C2[agam_jit Cranelift]
        C3[agam_runtime]
        C4[agam_profile]
    end

    Frontend --> Analysis --> Execution
```

1. **Frontend**: `agam_lexer` and `agam_parser` support `@lang.base`, `@lang.base.dynamic`, and `@lang.advance`.
2. **Semantic Layer**: `agam_sema` handles typing, effects, ownership, lifetimes, traits, and compile-time checks.
3. **IR Pipeline**: `agam_hir` lowers into `agam_mir`, which is the optimization and backend handoff layer.
4. **Native Backends**: `agam_codegen` emits C and LLVM IR. `agam_jit` uses Cranelift for in-memory execution.
5. **Runtime**: `agam_runtime` carries ARC, SIMD, hardware detection, and runtime helpers used by native execution paths.
6. **Diagnostics**: `agam_errors` is the canonical path for span-based reporting.
7. **Profiling**: `agam_profile` is the required benchmarking and optimization-validation path.

## 3. Implemented Features

1. **Language Modes**: Pythonic base syntax, dynamic base syntax, and advanced C-like syntax.
2. **Memory Model**: Hybrid ARC plus stricter lifetime-oriented modes.
3. **Native ML Primitives**: Tensor, autodiff, numerical routines, dataframe-style structures, and SIMD-aware runtime support.
4. **Advanced Static Analysis**: Effects, ownership, SMT-backed checking, and compile-time verification caches.
5. **Optimization Pipeline**: MIR constant folding, DCE, inlining, loop work, cache-alignment support, LLVM range/sign proof work, and JIT execution.
6. **Call Cache**:
   - Basic cache mode is opt-in and bounded.
   - Optimized cache mode uses bounded hot-entry replacement.
   - JIT admission now uses a fixed-size pending-candidate buffer instead of unbounded growth.

## 4. Platform and Performance Direction

1. **Portability Goal**: Java-like portability means a portable Agam package plus Agam runtime, not one identical native binary for every OS.
2. **Peak-Speed Goal**: LLVM/C AOT builds remain the path for target-specific maximum native speed.
3. **Runtime Goal**: JIT plus persistent native cache should remove repeated startup compilation and accelerate hot code adaptively.
4. **Safety Goal**: Performance features must preserve Agam semantics instead of depending on C/C++ undefined behavior contracts.

## 5. Engineering Rules

1. **WSL as Primary Linux Environment**: Use WSL Ubuntu 24.04 LTS for LLVM-adjacent, Unix-native, and Linux verification workflows. Keep Git add/commit on Win11. Do not store local credentials in tracked files.
2. **Crate-Level Isolation**: Modify the smallest responsible crate first and avoid unnecessary root-workspace blast radius.
3. **Mandatory Verification**: Before finalizing code generation work, run the relevant `cargo fmt --check` and `cargo check` commands from WSL, scoped to the affected crate(s) when possible.
4. **Phase-Based Atomic Commits**: Each completed feature phase should produce a conventional commit and a short Win11-side change summary.
5. **Zero-Destruction Testing**: Run unsafe FFI, pointer, and JIT-memory tests inside isolated subprocesses when possible.
6. **Zero-Cost Abstractions**: High-level constructs must lower into MIR and native backends without hidden steady-state overhead.
7. **Target Triangulation**: Design core `agam_std` modules so they can theoretically support native AOT, WASM, and JIT execution from one semantic model.
8. **No Silent Failures**: Avoid `.unwrap()` and `.expect()` in production compiler passes; route failures through `agam_errors`.
9. **AST/MIR Traceability**: Preserve `SourceId`, `Span`, and debug metadata through lowering and optimization.
10. **Benchmarking Mandate**: Before marking an optimization module complete, run a localized benchmark using `agam_profile` and record the measured delta.
11. **Zero-Regression Tolerance**: If an optimization increases compile time by more than 5 percent or reduces runtime speed against the current baseline, reject and rewrite that implementation before calling the phase complete.

## 6. Immediate Next Phases

1. **Phase 15A: Portable Agam Package + Tiered Runtime**
   - Define a platform-independent package format.
   - Load portable packages through the runtime and JIT hot code on the target machine.
2. **Phase 15B: Persistent Native Code Cache**
   - Add an on-disk native cache keyed by package hash, backend version, runtime ABI, OS, architecture, and CPU features.
   - Keep cache capacity bounded and invalidation version-aware.
3. **Phase 15C: Whole-Program Purity and Effect Metadata**
   - Prove purity and effects in the compiler.
   - Reuse that data for safer inlining, CSE, LICM, and auto-memoization.
4. **Phase 15D: Value Profiling + Adaptive Specialization**
   - Record hot argument/value patterns.
   - Specialize only when measured payoff is real.
5. **Phase 15E: Escape Analysis + Stack Promotion**
   - Move non-escaping values off the heap.
   - Reduce ARC and allocation pressure on hot paths.
6. **Phase 15F: Incremental Daemon + Parallel Compilation**
   - Keep typed/lowered state warm across edits.
   - Parallelize independent compiler work with deterministic diagnostics.
