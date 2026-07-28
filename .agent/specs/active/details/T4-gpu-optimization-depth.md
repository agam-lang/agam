# Phase T4-gpu-optimization-depth � GPU and NPU Optimization Depth

**Status:** partial (finish remaining Phase T3-gpu-npu-pipeline items)
**Tier:** 4 (Performance and Optimization Depth)

## Scope

Complete the remaining Phase T3-gpu-npu-pipeline GPU integration items: rich memory types, real CUDA kernel launch lowering, and multi-backend GPU support.

## Remaining Deliverables

### Safe GPU Memory Types (inspired by cuda-oxide's DisjointSlice)
- [ ] `GpuBuffer<T>` — typed, bounds-checked device buffer (replaces raw `*f32` kernel params)
- [ ] `SharedMem<T, N>` — compile-time-sized shared memory with `addrspace(3)` lowering
- [ ] `WarpSlice<T>` — warp-cooperative memory view for shuffle/reduce patterns
- [ ] Pointer/array lowering in GPU kernels using typed buffers instead of annotated pointers
- [ ] Sema validation: reject raw pointer arithmetic in kernels when typed buffers are available

### Kernel Launch and Execution
- [ ] Replace host-side kernel-launch placeholder with real CUDA launch lowering
- [ ] Multi-dimensional grid/block configuration (`@gpu(grid: [G1,G2], block: [B1,B2,B3])`)
- [ ] Occupancy analysis and auto-tuning (query device SM count, shared mem limits)
- [ ] Lazy GPU compilation: skip PTX emission for `@gpu` functions not transitively called

### Warp-Level Primitives
- [ ] `warp_shuffle(val, lane)` — cross-lane data exchange
- [ ] `warp_vote(predicate)` — warp ballot/any/all
- [ ] `warp_reduce(val, op)` — warp-level reduction (sum, min, max)
- [ ] Map to NVVM `__shfl_sync`, `__ballot_sync`, `__reduce_sync` intrinsics
- [ ] Warp Specialization (`@gpu(warp_specialize)`) for latency hiding and divergent control flow (AutoWS)

### Multi-Backend GPU Support
- [ ] Integrate LLVM's new unified offload driver as the primary target for `@gpu` compilation
- [ ] Test device-side LTO compatibility using the offload driver
- [ ] Triton/MLIR-based NPU backend compilation (e.g., Hexagon-MLIR) for generating TCM-optimized megakernels
- [ ] SPIR-V backend for Vulkan/OpenCL targets (stretch)
- [ ] GPU-MIR abstraction layer to share logic between NVPTX and SPIR-V emitters

## Already Completed (from Phase 23)

- ✅ `@gpu` kernel config, sema validation, MIR ops
- ✅ NVPTX64 IR emitter with CUDA linkage
- ✅ GPU builtins, shared memory, math intrinsics
- ✅ Host-device transfer APIs
- ✅ 29+ GPU-focused tests, 713+ total tests passing

## Design References

- **cuda-oxide (NVIDIA)**: Single-source Rust→PTX compiler using Pliron IR with dialect-mir/dialect-llvm/dialect-nvvm layers. Key idea: `DisjointSlice<T>` type-safe GPU memory model prevents data races at the type level. Lazy device compilation only generates PTX for transitively-called kernels.
- **LLVM Unified Offload Driver**: LLVM 22+ has unified its offloading driver (used by CUDA, HIP, and OpenMP) providing seamless device-side LTO and static library support.
- **Agam advantage**: Agam's `@gpu` annotation is a first-class language feature, not a compiler plugin. This gives deeper sema/type integration than cuda-oxide's `#[kernel]` attribute approach.

## Responsible Crates

- `agam_sema` — GPU validation updates, typed buffer checking
- `agam_codegen` — GPU emitter enhancements, SPIR-V backend
- `agam_std` — host-device API expansion, warp primitives

## Dependencies

- Phase T0-type-system (type system) — const generics needed for `SharedMem<T, N>` and `GpuBuffer<T>`
- Phase T0-object-model (object model) — operator overloading for buffer indexing
