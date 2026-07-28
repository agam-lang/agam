# Phase T3-spirv-backend � Vendor-Neutral GPU Backend via SPIR-V

## Goal

Provide a robust, vendor-neutral GPU execution backend for Agam-Lang's `@gpu` (Avyayībhāva) annotated blocks, targeting SPIR-V.

## Background

Agam-Lang treats AI, tensor, and numerical workflows as first-class language concerns. Instead of implementing separate backends for CUDA (NVIDIA) and HIP (AMD), this phase focuses on generating SPIR-V as a universal intermediate representation for GPU compute.

The release of chipStar 1.3 proves that SPIR-V has matured to fully support complex CUDA/HIP kernels, allowing them to be executed on platforms providing OpenCL or Intel Level Zero runtimes. We will leverage SPIR-V to ensure our tensor workflows remain performant and portable across hardware from NVIDIA, AMD, Intel, and Apple.

## Implementation Checklist

- [ ] **IR Lowering:** Define the lowering pass from Agam MIR to LLVM IR specifically configured for the SPIR-V target triple (`spirv64-unknown-unknown`).
- [ ] **Backend Target Integration:** Integrate the LLVM SPIR-V backend into `agam_codegen`.
- [ ] **Runtime Dispatch:** Implement a host-side runtime component in `agam_runtime` to load and execute the generated SPIR-V binaries via OpenCL, Vulkan, or Level Zero depending on the host hardware.
- [ ] **Memory Management:** Map Agam's tensor memory model to GPU memory spaces (global, shared, local) in the SPIR-V lowering.
- [ ] **Validation:** Create an end-to-end benchmark suite for `@gpu` tensor operations and validate execution correctness on at least two distinct GPU architectures (e.g., NVIDIA via OpenCL and Intel/AMD).

## Dependencies

- Phase T3-gpu-npu-pipeline (GPU and NPU Integration - Advanced)
- Phase T1-sdk-distribution (Native LLVM SDK Distribution)
