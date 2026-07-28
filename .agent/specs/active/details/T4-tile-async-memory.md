# Phase T4-tile-async-memory — Tile-Centric Programming Model and Asynchronous Memory

## Goal

Abstract the low-level SIMT (Single Instruction, Multiple Thread) execution model for Agam-Lang's `@gpu` kernels, enabling developers to write single-threaded logical code that the compiler automatically parallelizes across physical threads and memory accelerators (like Hopper's TMA).

## Background

Historically, GPU programming required manual management of thread indices (`threadIdx.x`), block layouts, and explicit data loading loops to move memory from global VRAM to fast Shared Memory. 

Building upon the concepts introduced in Phase T4-gpu-auto-tuning (Tile Abstractions) and deeply inspired by NVIDIA's `CUDA Tile for C++`, Phase T4-tile-async-memory fundamentally changes the semantic lowering of Agam's `@gpu` functions. The developer writes code as if it executes on a **single thread per block**. The compiler assumes the responsibility of decomposing the high-level `Tile` operations into thousands of physical thread instructions and utilizing asynchronous memory transfer hardware (TMA).

## Implementation Checklist

- [ ] **Single-Thread Illusion:** Modify the `@gpu` lowering pass so that standard block loops and operations are automatically striped across the physical thread block. The user writes logic for one "logical block", and the compiler generates the thread grid mapping.
- [ ] **Multi-dimensional Data Views:** Implement `agam_std::gpu::Extent` and `PartitionView` types to safely represent slices of multi-dimensional tensors without manual pointer arithmetic.
- [ ] **Asynchronous Memory Transfers:** When moving a `PartitionView` from global memory into a `Tile` (shared memory), the compiler should emit SPIR-V instructions that trigger asynchronous memory copies (e.g., mapping to TMA or standard async `memcpy_async` instructions), bypassing the CPU/ALU.
- [ ] **Validation:** Write a `@gpu` kernel that loads a tile, processes it, and stores it back. Verify that the generated SPIR-V contains the correct thread-id scaling and async memory wait operations, even though the source Agam code contained no thread indices or manual synchronization barriers.

## Dependencies

- Phase T4-gpu-auto-tuning (GPU Auto-Tuning and Tile Abstractions)
- Phase T3-spirv-backend (Vendor-Neutral GPU Backend via SPIR-V)
