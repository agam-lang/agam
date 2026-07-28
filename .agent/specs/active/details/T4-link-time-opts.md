# Phase T4-link-time-opts � Link-Time Whole-Program Optimizations

**Status:** open
**Tier:** 4 (Performance and Optimization)

## Scope

Establish a multi-vendor GPU compilation pipeline by generating SPIR-V intermediate representation. This will enable Agam `@gpu` kernels to execute on AMD, Intel, ARM Mali, and Apple Silicon hardware via OpenCL and Intel Level Zero runtimes, breaking the current NVPTX/CUDA lock-in.

## Deliverables

### SPIR-V Emitter
- [ ] Implement a SPIR-V backend in `agam_codegen`
- [ ] Map Agam GPU builtins (thread_id, block_id, etc.) to SPIR-V execution model
- [ ] Map math intrinsics and shared memory to SPIR-V standard library
- [ ] Share abstraction layer with NVPTX emitter (GPU-MIR)

### Runtime and Toolchain Integration
- [ ] Integrate OpenCL / Level Zero runtime dispatch for host-side kernel launches
- [ ] Adapt the chipStar compilation flow (HIP/CUDA → SPIR-V) as a reference model for Agam's lowering strategy
- [ ] Implement multi-target fat-binary bundling (NVPTX + SPIR-V)

## Design References
- **chipStar 1.3**: Uses LLVM SPIR-V translation to allow HIP and CUDA programs to run on OpenCL and Level Zero. Validates the viability of SPIR-V as a universal distribution format for compute kernels.
- **PoCL (Portable Computing Language)**: Specifically noted for enabling OpenCL on Apple Silicon (ARM64) and x86_64, which is relevant for Agam's cross-platform goals.

## Dependencies
- Phase T4-gpu-optimization-depth (GPU/NPU Completion) — completes the base `@gpu` infrastructure and GPU-MIR abstraction layer.
