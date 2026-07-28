# Phase T5-tensor-core-matrix — Tensor Core and Cooperative Matrix Integration

## Goal

Provide native, vendor-neutral hardware acceleration for tensor multiplication workflows in Agam-Lang by mapping language-level primitives directly to GPU Tensor/Matrix Cores via SPIR-V's `SPV_KHR_cooperative_matrix` extension.

## Background

Machine learning and AI inference workloads heavily rely on matrix multiplication and accumulation. Modern GPUs and NPUs (NVIDIA, AMD, Intel, ARM) provide specialized hardware acceleration units (e.g., Tensor Cores) for these operations.

Historically, languages have relied on massive, vendor-specific SDKs (like cuDNN or oneDNN) to access these units. With the introduction of the `cl_khr_cooperative_matrix` OpenCL extension and the corresponding `SPV_KHR_cooperative_matrix` SPIR-V extension, Agam-Lang can generate vendor-neutral GPU binaries that directly target these hardware matrix units. This fulfills Agam's core promise of treating tensor workflows as first-class, high-performance language primitives.

## Implementation Checklist

- [ ] **Type Mapping:** Define the lowering of Agam's standard tensor types (e.g., `tensor<f16, 16x16>`) to SPIR-V Cooperative Matrix types during the MIR-to-LLVM lowering pass.
- [ ] **Intrinsic Generation:** Map core Agam tensor multiplication and accumulation operators (`*`, `+` in tensor contexts) to the `OpCooperativeMatrixMulAddKHR` SPIR-V instruction within `@gpu` annotated blocks.
- [ ] **Runtime Query API:** Extend `agam_runtime` to expose the new device query APIs (checking for supported matrix sizes, data types, and capabilities on the host hardware).
- [ ] **Dynamic Dispatch & Fallback:** Implement a runtime dispatch layer that gracefully degrades to standard SIMD/vectorized multiplication if the host device lacks Cooperative Matrix support or if the requested matrix sizes are incompatible.
- [ ] **Validation:** Build an inference benchmark (e.g., a simple transformer layer or MLP) and validate its performance against raw `clang++` OpenCL equivalents, ensuring the hardware Tensor Cores are successfully activated.

## Dependencies

- Phase T3-spirv-backend (Vendor-Neutral GPU Backend via SPIR-V)
- Phase T3-gpu-npu-pipeline (GPU and NPU Integration - Advanced)
