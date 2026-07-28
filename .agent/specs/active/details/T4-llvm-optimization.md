# Phase T4-llvm-optimization � Advanced LLVM Optimization

**Status:** open
**Tier:** 4 (Performance and Optimization Depth)

## Scope

Build on Agam's existing strong LLVM backend to add production-quality PGO, LTO, auto-vectorization, and advanced loop optimizations. Target LLVM 22.1+ for maximum codegen quality.

## LLVM Version Strategy

> **Action**: Upgrade LLVM target from 20.x to 22.1.
>
> SPEC CPU 2026 benchmarks (May 2026) demonstrate LLVM 22 delivers 5-8% geomean improvement over LLVM 20 across Intel, AMD, and NVIDIA/ARM platforms. Individual workloads see up to 129% improvement (sealcrypto — homomorphic encryption, vector/SIMD-heavy) and 110% improvement (marian — neural machine translation, loop optimization).
>
> LLVM 22.1 (February 2026) introduced the new `ptrtoaddr` IR instruction which enables better alias analysis by stripping provenance.
>
> Since Agam targets numerical/ML workloads, these are exactly the optimization patterns that benefit Agam's output quality.

- [ ] Validate `agam_codegen` LLVM IR emission against LLVM 22.1 APIs
- [ ] Evaluate `ptrtoaddr` instruction for pointer casting where provenance is not needed
- [ ] Update `agamc doctor` to detect and recommend LLVM 22.1+
- [ ] Update SDK distribution (Phase T1-sdk-distribution) to bundle LLVM 22.1 toolchain
- [ ] Benchmark Agam's output quality on LLVM 20 vs LLVM 22.1 (use `benchmarks/` suite)
- [ ] LLVM 23 IR compatibility: migrate floating-point literal emission to the new `f0x` prefix and adopt `denormal_fpenv` attributes for numerical correctness

## Deliverables

### Profile-Guided Optimization (PGO)
- [ ] Complete the `--pgo-generate` → `--pgo-use` loop into a real workflow
- [ ] Profile data collection, merging (`llvm-profdata merge`)
- [ ] PGO-aware inlining and branch prediction
- [ ] `agamc build --pgo-generate` and `agamc build --pgo-use profile.profdata`

### Link-Time Optimization (LTO)
- [ ] Thin LTO for fast builds with cross-module optimization
- [ ] Full LTO for maximum optimization at the cost of build time
- [ ] `--lto thin` and `--lto full` flags
- [ ] Evaluate ThinLTO MTPC (Multi-Thread Parallel Compilation) for intra-module parallelism (`--lto thin-parallel`)
- [ ] Add DTLTO (Distributed ThinLTO) awareness for offloading LLD linker tasks

### Auto-Vectorization
- [ ] SIMD auto-vectorization for eligible loops
- [ ] SIMD intrinsic fallback for performance-critical code
- [ ] Target-specific SIMD: SSE4, AVX2, AVX-512, NEON
- [ ] Vectorization diagnostics: report why a loop wasn't vectorized

### Loop Optimizations
- [ ] Loop tiling for cache optimization
- [ ] Loop interchange for memory access patterns
- [ ] Loop fusion to reduce overhead
- [ ] Loop distribution for parallelization opportunities

### Escape Analysis
- [ ] Stack-allocate heap objects that don't escape the current scope
- [ ] Elide ARC/reference counting for non-escaping references

### Devirtualization
- [ ] Whole-program analysis to convert dynamic dispatch to static
- [ ] Speculative devirtualization with guard checks

## Design References

- **SPEC CPU 2026 on LLVM 22 (ServeTheHome, May 2026)**: Compiler upgrades deliver measurable, benchmark-verified performance gains. Key insight: auto-vectorization and loop optimization improvements in LLVM 22 disproportionately benefit numerical, crypto, and ML workloads — Agam's core domain.
- **LLVM 22.1 Release (February 2026)**: Stable release candidate introducing `ptrtoaddr`, DTLTO linker improvements, and MTPC intra-module parallelism proposal.
- **SPEC CPU run rules**: Compiler optimizations must benefit real-world code, not just benchmarks. Agam's optimization passes should follow the same principle.

## Responsible Crates

- `agam_mir` — MIR-level optimization passes
- `agam_codegen` — LLVM pass pipeline configuration, LLVM version compatibility
- `agam_driver` — optimization flags and PGO workflow
