# Memory Model Status

**Decision made. Implementation pending.**

> See Phase T2-memory-model (`specs/active/details/T2-memory-model.md`) for the full design options analysis.

## Decision: Hybrid ARC + Value Semantics

The recommended and decided memory model for Agam is **Hybrid ARC + Value Semantics (Option B+)**:

| Aspect | Decision |
|--------|----------|
| **Default** | Value semantics for structs (copy-by-default, like Swift) |
| **Heap allocation** | ARC (Automatic Reference Counting) for reference types |
| **ARC optimization** | Escape analysis to elide ARC operations in hot paths |
| **Zero-copy hot paths** | Optional `@move` annotation for explicit move semantics |
| **Borrow checker** | **No** mandatory borrow checker; optional `@strict` mode for systems code |
| **Safety guarantee** | No use-after-free via ARC; no null via `Option<T>` |

## Rationale

- **Rust-style borrow checking** was rejected — fights the "Python readability" promise; borrow checker is notoriously hard to implement and use
- **GC** was rejected — conflicts with Agam's real-time/embedded/HPC targets
- **Pure ARC** (Swift) is the base, but enhanced with escape analysis and value semantics

## What Is Implemented Today (May 2026)

- ❌ **Nothing** — the memory model decision is made but not implemented
- Current code works because programs are simple (scalars, effects, basic structs) and no ownership conflicts arise
- There is no ARC insertion, no escape analysis, no lifetime tracking in `agam_sema`

## What Needs To Be Built (Phase T2-memory-model)

1. **Sema pass:** ownership/lifetime analysis — distinguish stack vs heap types
2. **HIR:** representation of memory operations (alloc, dealloc, ARC inc/dec)
3. **MIR optimization:** elide redundant ARC operations (escape analysis)
4. **Codegen:** emit allocation, deallocation, reference counting in both backends
5. **Runtime:** ARC runtime support in `agam_runtime`
6. **Stdlib alignment:** collection types and smart pointer types

## Interaction With Other Phases

- **T0-type-system** — `Option<T>` requires generics; must land before null-safety is enforceable
- **T0-object-model** — struct semantics (copy vs reference) depend on memory model
- **T2-async-concurrency** — data race freedom in concurrent code integrates with ownership
- **T2-cybersecurity** — use-after-free and double-free prevention are part of the security story
- **T3-universal-ffi** — ownership interop with C/Python/Rust across FFI boundaries

## Key Rule of Thumb

**Do not implement GC.** Any proposal to add garbage collection should be rejected unless the target is explicitly a managed-runtime profile (e.g., a future JVM or CLR backend). The native, embedded, and HPC targets explicitly require deterministic memory management.
