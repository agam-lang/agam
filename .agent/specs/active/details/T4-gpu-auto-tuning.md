# Phase T4-gpu-auto-tuning — GPU Auto-Tuning and Tile Abstractions

## Goal

Enhance Agam-Lang's `@gpu` execution pipeline by introducing an evolutionary compiler auto-tuning framework (`agam_mir::opt` tuner) and high-level `Tile` abstractions to simplify complex shared-memory orchestrations.

## Background

While Agam targets vendor-neutral SPIR-V rather than proprietary CUDA APIs, modern GPU architectures (like NVIDIA Hopper, AMD CDNA) require extremely delicate orchestration of shared memory, thread block synchronization, and register allocation to achieve peak AI/tensor performance.

Inspired by the release of CUDA 13.3 (which introduced `CompileIQ` for genetic algorithm-based compiler auto-tuning, and `CUDA Tile for C++` for high-level memory abstraction), Agam will adopt these concepts natively. This ensures our language remains not just vendor-neutral, but highly optimized and ergonomic for developers writing raw tensor math.

## Implementation Checklist

- [ ] **Evolutionary Auto-Tuner:** Introduce an `agamc build --tune-gpu` mode. This mode will use a genetic algorithm to permute SPIR-V lowering strategies, unroll factors, and MIR optimization passes, benchmarking each iteration to find the absolute fastest configuration for a given `@gpu` kernel.
- [ ] **Tile Type Abstraction:** Introduce `agam_std::gpu::Tile<T, N>` into the standard library. This type will abstract the low-level indexing of shared memory banks and thread synchronization.
- [ ] **Semantic Lowering of Tiles:** Teach the compiler to lower `Tile` operations implicitly into the correct barrier and memory fence SPIR-V instructions.
- [ ] **MLIR Integration Investigation:** Research whether an optional MLIR pipeline step within Agam's lowering (before emitting LLVM IR/SPIR-V) could yield faster JIT compile times for dynamically generated tensor kernels, similar to the Numba CUDA MLIR backend.

## Dependencies

- Phase T5-tensor-core-matrix (Tensor Core & Cooperative Matrix Integration)
- Phase T3-gpu-npu-pipeline (GPU and NPU Integration - Advanced)
