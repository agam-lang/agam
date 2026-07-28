# Phase T4-incremental-compile � Incremental Compilation

**Status:** open (was Phase 24)
**Tier:** 4 (Performance and Optimization Depth)

## Scope

Make Agam's native scientific and data-structure surface competitive and broad. Sparse matrices, FFT, signal processing, and high-performance collections.

## Deliverables

- [ ] Sparse matrix hardware-aware types for large NLP and graph workloads
- [ ] Native FFT and signal-processing support in `agam_std::math`
- [ ] High-performance `agam_std::collections` with lock-free and vectorized structures (B-Trees, graphs, fast maps)
- [ ] BLAS/LAPACK-competitive linear algebra primitives

## Responsible Crates

- `agam_std`

## Dependencies

- Phase T0-type-system (type system) — generics needed for collection types
- Phase T2-memory-model (memory model) — collections need clear ownership semantics
- Runtime SIMD and optimizer groundwork
