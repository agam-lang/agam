# Phase T3-gpu-target-adapter — Universal GPU Target Adapter Interface

**Status:** complete

## Scope

Abstract GPU code generation into a universal target adapter interface decoupling GPU MIR lowering from hardware assembly emission, supporting AMDGPU (ROCm/HIP), SPIR-V (Vulkan/oneAPI), and Apple Metal (MSL/AIR) alongside NVPTX64.

## Deliverables

### Abstract Adapter Trait (`agam_codegen::gpu_adapter`)
- [x] `GpuTargetKind` enum: `Nvptx`, `Amdgpu`, `Spirv`, `Metal`
- [x] `GpuTargetAdapter` trait:
  - `target_triple()`
  - `target_datalayout()`
  - `kernel_calling_convention()`
  - `shared_memory_addrspace()`
  - `emit_intrinsics_header()`
  - `map_intrinsic_symbol()`
  - `emit_barrier()`
  - `emit_warp_shuffle_down()`
  - `emit_host_runtime_declarations()`
  - `linker_flags()`
- [x] Concrete Target Adapters:
  - `NvptxAdapter`: NVPTX64 CUDA target (`nvptx64-nvidia-cuda`, `ptx_kernel`, `@llvm.nvvm.read.ptx.sreg.tid.x`)
  - `AmdgpuAdapter`: AMD ROCm/HIP GCN/RDNA target (`amdgcn-amd-amdhsa`, `amdgpu_kernel`, `@llvm.amdgcn.workitem.id.x`)
  - `SpirvAdapter`: Vulkan / oneAPI / OpenCL target (`spirv64-unknown-unknown`, `spir_kernel`, `@__spirv_BuiltInLocalInvocationId`)
  - `MetalAdapter`: Apple Silicon Metal Shading Language target (`air64-apple-macosx`, `metal_kernel`, `@air.thread_position_in_threadgroup.x`)
- [x] Target resolution helpers: `adapter_for_target`, `adapter_from_triple`

### Emitter Integration (`agam_codegen::gpu_emitter`)
- [x] `emit_gpu_module_with_adapter`: Target-agnostic GPU kernel emission
- [x] `emit_gpu_module_for_target`: Direct emission by `GpuTargetKind`
- [x] `emit_gpu_module`: NVPTX default with 100% backward compatibility

### Verification Suite (`agam_test::gpu_output`)
- [x] AMDGPU target kernel and intrinsic verification
- [x] SPIR-V target compute kernel and builtin verification
- [x] Apple Metal target AIR kernel and threadgroup barrier verification
- [x] Target triple resolution and linker flag validation

## Test Results
- 91/91 tests pass in `agam_test`
- 100% test pass rate across all 27 crates in workspace
- 0 Clippy warnings (`-D warnings`)
- 100% formatting compliance (`cargo fmt --check`)
