# Phase Dependency Map

Which phases must (or should) complete before others can begin. Use this before starting any new phase to understand what needs to be in place.

## Tier Flow (Hard Dependencies)

```
┌─────────────────────────────────────────────────────────────┐
│  TIER 0 — Foundation                                        │
│  T0-type-system, T0-object-model, T0-module-system,        │
│  T0-effects-depth, T0-grammar-spec, T0-stdlib-completion    │
└──────────────────────────┬──────────────────────────────────┘
                           │ (Tier 1 can start in parallel)
           ┌───────────────┼────────────────────┐
           ▼               ▼                    ▼
┌──────────────────┐ ┌──────────────────┐  (T0 complete)
│  TIER 1 — DX    │ │  TIER 2 — Runtime│
│  T1-error-msgs  │ │  T2-memory-model │
│  T1-lsp-prod    │ │  T2-async-concur │
│  T1-build-sys   │ │  T2-cybersecurity│
│  ...            │ │  ...             │
└──────────────────┘ └────────┬─────────┘
                              │
                              ▼
               ┌──────────────────────────┐
               │  TIER 3 — Platform       │
               │  T3-wasm-backend         │
               │  T3-cross-compilation    │
               │  T3-native-renderer      │
               │  T3-universal-ffi        │
               │  ...                     │
               └──────────┬───────────────┘
                          │
                          ▼
               ┌──────────────────────────┐
               │  TIER 4 — Optimization   │
               │  T4-llvm-optimization    │
               │  T4-multi-level-ir       │
               │  T4-metaprogramming      │
               │  ...                     │
               └──────────┬───────────────┘
                          │
                          ▼
               ┌──────────────────────────┐
               │  TIER 5 — AI-Native      │
               │  T5-autodiff-tensors     │
               │  T5-edge-ai-inference    │
               │  T5-probabilistic-prog   │
               │  ...                     │
               └──────────┬───────────────┘
                          │
                          ▼
               ┌──────────────────────────┐
               │  TIER 6 — Frontier       │
               │  T6-self-hosting         │
               │  T6-formal-verification  │
               │  T6-ai-compiler-intel    │
               └──────────────────────────┘
```

## Specific Phase Dependencies

### T0-type-system depends on:
- T0-grammar-spec (grammar should be specified first or in parallel)

### T0-object-model depends on:
- T0-type-system (generics needed for trait bounds)
- T0-module-system (visibility rules need module boundaries)

### T0-effects-depth depends on:
- T0-type-system (closures need type inference; operators need traits)
- T0-object-model (operator overloading needs trait system)

### T1-error-messages depends on:
- Independent of other Tier 0 work — can start during T0-type-system

### T1-lsp-production depends on:
- T0-type-system (completion needs type information)
- T0-object-model (hover needs method resolution)

### T1-advanced-testing depends on:
- T1-testing-framework (basic `agamc test` infrastructure)
- T1-doc-generation (doctest extraction)

### T1-compiler-agent-tool depends on:
- Phase 18 (agent-facing execution) — extends existing `agamc exec` contract
- Phase 19 (Python wrappers) — extends existing Python integration
- T1-lsp-production (LSP must be functional first)

### T2-memory-model depends on:
- T0-type-system (generics needed for `Option<T>`, `Arc<T>` smart pointers)
- T0-object-model (struct semantics depend on memory model)

### T2-async-concurrency depends on:
- T2-memory-model (concurrency safety depends on memory safety)
- T0-type-system (`Future<T>` needs generics)

### T2-cybersecurity depends on:
- T2-memory-model (memory safety guarantees)
- T0-type-system (`Secret<T>`, `Option<T>` capability types)
- T0-object-model (traits for crypto abstractions)
- T0-module-system (visibility rules for capability enforcement)

### T3-wasm-backend depends on:
- T0-type-system, T0-object-model (stable type layouts)
- T0-module-system (module interfaces → WIT exports)
- T2-async-concurrency (WASI 0.3 async components)

### T3-native-renderer depends on:
- T0-type-system, T0-object-model (widget traits need generics)
- T3-wasm-backend (web rendering target)

### T3-universal-ffi depends on:
- T0-type-system (generics for typed FFI wrappers)
- T2-memory-model (ownership interop with foreign languages)
- T3-wasm-backend (JavaScript/web interop)

### T4-llvm-optimization depends on:
- Phase 15H (LLVM SDK distribution — already partial)

### T4-multi-level-ir depends on:
- T0-type-system (const generics for tensor shapes)
- T4-gpu-optimization-depth (GPU emitter needs dialect refactoring)
- T5-autodiff-tensors (AD transforms benefit from tensor dialect)

### T5-autodiff-tensors depends on:
- T0-type-system (const generics for tensor shapes)
- T0-object-model (operator overloading for tensor math)
- T4-gpu-optimization-depth (GPU-accelerated tensor operations)
- T4-multi-level-ir (tensor dialect for fusion and layout optimization)

### T5-edge-ai-inference depends on:
- T5-autodiff-tensors (tensor types and operations)
- T4-multi-level-ir (tensor dialect for operator fusion)
- T4-gpu-optimization-depth (GPU-accelerated inference kernels)
- T0-type-system (const generics for model shapes)

### T6-self-hosting depends on:
- Tiers 0–2 substantially complete
- Standard library must support file I/O, strings, collections

### T6-formal-verification depends on:
- Tiers 0–2 complete (stable type system and memory model)

## In-Flight Phase Dependencies (Numbered)

| In-flight phase | Hard deps | Detail |
|-----------------|-----------|--------|
| Phase 23 (GPU) | Phase 15H (LLVM SDK) | Needs bundled LLVM toolchain |
| Phase 15H | Phase 17 family (pkg) | SDK publish needs registry |
| Phase 18 (exec) | Phase 21 (sandbox) | Isolation built on OS sandbox |
| Phase 19 (wrappers) | Phase 18 (exec) | Uses `agamc exec --json` |
| T1-compiler-agent-tool | Phase 18, Phase 19, T1-lsp-production | Unifies all three |
