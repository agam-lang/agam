# Chapter 33: SPIR-V Backend & Vendor-Neutral GPU Compilation

> **Part VIII: GPU, Hardware Acceleration & AI-Native Infrastructure**  
> **Compiler Module Focus**: [`agam_codegen::spirv`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/backends/agam_codegen/src/spirv.rs), [`agam_codegen::gpu_adapter`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/backends/agam_codegen/src/gpu_adapter.rs)

---

## 33.1 Why SPIR-V?

SPIR-V (Standard Portable Intermediate Representation) is the Khronos Group's binary intermediate language for parallel compute and graphics. Agam uses SPIR-V as its **primary vendor-neutral GPU backend** for several architectural reasons:

| Approach | Vendor Lock-In | Runtime Support | Agam's Choice |
| :--- | :---: | :--- | :---: |
| CUDA PTX | NVIDIA only | CUDA Runtime | ❌ Vendor-locked |
| Metal Shading Language | Apple only | Metal Framework | ❌ Vendor-locked |
| **SPIR-V** | **Vendor-neutral** | **Vulkan, OpenCL, Level Zero** | **✅ Primary** |
| NVPTX via adapter | NVIDIA | CUDA | ✅ Secondary |
| Metal via adapter | Apple | Metal | ✅ Secondary |

A single Agam `@gpu` kernel compiles to SPIR-V once and runs on **any** GPU supporting Vulkan Compute, OpenCL 2.0+, or Intel Level Zero — including NVIDIA, AMD, Intel, Qualcomm, and ARM Mali GPUs.

---

## 33.2 SPIR-V Module Architecture

The Agam SPIR-V emitter (`agam_codegen::spirv`) generates compliant SPIR-V 1.5 binary modules:

```text
SPIR-V Binary Module Layout:
  ┌──────────────────────────────────────────────────┐
  │ Magic Number: 0x07230203                          │
  │ Version: 1.5                                      │
  │ Generator ID: Agam Compiler                       │
  │ Bound: (max ID + 1)                               │
  ├──────────────────────────────────────────────────┤
  │ 1. Capability Declarations                        │
  │    OpCapability Shader                            │
  │    OpCapability Float64                           │
  │    OpCapability CooperativeMatrixKHR              │
  ├──────────────────────────────────────────────────┤
  │ 2. Extension Imports                              │
  │    OpExtInstImport "GLSL.std.450"                 │
  │    OpExtension "SPV_KHR_cooperative_matrix"       │
  ├──────────────────────────────────────────────────┤
  │ 3. Memory Model                                   │
  │    OpMemoryModel Logical GLSL450                  │
  ├──────────────────────────────────────────────────┤
  │ 4. Entry Points                                   │
  │    OpEntryPoint GLCompute %main "main" %gl_GlobalInvocationID │
  │    OpExecutionMode %main LocalSize 256 1 1        │
  ├──────────────────────────────────────────────────┤
  │ 5. Type Declarations                              │
  │    %float = OpTypeFloat 32                        │
  │    %v4float = OpTypeVector %float 4               │
  │    %mat4 = OpTypeMatrix %v4float 4                │
  │    %ptr_ssbo = OpTypePointer StorageBuffer %float │
  ├──────────────────────────────────────────────────┤
  │ 6. Variable Declarations (Descriptor Bindings)    │
  │    %input_a = OpVariable %ptr_ssbo StorageBuffer  │
  │    %input_b = OpVariable %ptr_ssbo StorageBuffer  │
  │    %output  = OpVariable %ptr_ssbo StorageBuffer  │
  ├──────────────────────────────────────────────────┤
  │ 7. Function Definitions                           │
  │    %main = OpFunction ...                         │
  │    (kernel body instructions)                     │
  │    OpReturn / OpFunctionEnd                       │
  └──────────────────────────────────────────────────┘
```

---

## 33.3 MIR to SPIR-V Lowering

The SPIR-V emitter translates optimized GPU MIR dialect operations into SPIR-V instructions:

| Agam MIR Operation | SPIR-V Instruction |
| :--- | :--- |
| `GpuThreadId(X)` | `OpLoad %gl_GlobalInvocationID` + `OpCompositeExtract 0` |
| `GpuBlockId(Y)` | `OpLoad %gl_WorkGroupID` + `OpCompositeExtract 1` |
| `GpuSyncThreads` | `OpControlBarrier Workgroup Workgroup AcquireRelease` |
| `Add(a, b)` | `OpFAdd` / `OpIAdd` |
| `Mul(a, b)` | `OpFMul` / `OpIMul` |
| `Load(ptr, idx)` | `OpAccessChain` + `OpLoad` |
| `Store(ptr, idx, val)` | `OpAccessChain` + `OpStore` |
| `Branch(cond, t, f)` | `OpBranchConditional` |
| `AtomicAdd(ptr, val)` | `OpAtomicIAdd` / `OpAtomicFAddEXT` |

### Example: Vector Add Lowering

```agam
// Source
@gpu
fn vector_add(a: Tensor[Float], b: Tensor[Float], out: Tensor[Float]) {
    let idx = gpu.thread_id();
    out[idx] = a[idx] + b[idx];
}
```

```text
// Generated SPIR-V (disassembled)
%main = OpFunction %void None %void_fn
%entry = OpLabel

; Get global thread ID
%gid_ptr = OpAccessChain %ptr_input_uint %gl_GlobalInvocationID %uint_0
%gid = OpLoad %uint %gid_ptr

; Load a[idx]
%a_ptr = OpAccessChain %ptr_ssbo_float %input_a %uint_0 %gid
%a_val = OpLoad %float %a_ptr

; Load b[idx]
%b_ptr = OpAccessChain %ptr_ssbo_float %input_b %uint_0 %gid
%b_val = OpLoad %float %b_ptr

; Compute a[idx] + b[idx]
%sum = OpFAdd %float %a_val %b_val

; Store to out[idx]
%out_ptr = OpAccessChain %ptr_ssbo_float %output %uint_0 %gid
OpStore %out_ptr %sum

OpReturn
OpFunctionEnd
```

---

## 33.4 Tensor Core Acceleration via `SPV_KHR_cooperative_matrix`

For matrix multiplication workloads, the SPIR-V emitter leverages the `SPV_KHR_cooperative_matrix` extension to access hardware tensor cores (NVIDIA Tensor Cores, Intel XMX, AMD Matrix Cores):

```text
Cooperative Matrix SPIR-V Flow:

  1. OpCooperativeMatrixLoadKHR    — Load tile from global memory
  2. OpCooperativeMatrixMulAddKHR  — Hardware matrix multiply-accumulate
  3. OpCooperativeMatrixStoreKHR   — Store result tile to global memory
```

### Compiler-Generated Tensor Core Kernel

When the compiler detects a matrix multiplication pattern, it automatically generates cooperative matrix instructions:

```text
// For C = A × B where A is MxK, B is KxN
// Using 16×16×16 cooperative matrix tiles

%tile_a = OpCooperativeMatrixLoadKHR %coop_mat_a %ptr_A %stride_A RowMajor
%tile_b = OpCooperativeMatrixLoadKHR %coop_mat_b %ptr_B %stride_B ColumnMajor
%tile_c = OpCooperativeMatrixLoadKHR %coop_mat_c %ptr_C %stride_C RowMajor

; Hardware tensor core MMA: C += A × B
%result = OpCooperativeMatrixMulAddKHR %coop_mat_c %tile_a %tile_b %tile_c

OpCooperativeMatrixStoreKHR %ptr_C %result %stride_C RowMajor
```

**Performance impact:** Cooperative matrix operations execute on dedicated tensor core hardware at up to **312 TFLOPS** (FP16) on NVIDIA H100, versus **60 TFLOPS** for standard CUDA cores.

---

## 33.5 Runtime Dispatch: Vulkan / OpenCL / Level Zero

The compiled SPIR-V binary is dispatched to the GPU through the available compute runtime:

```text
SPIR-V Module (.spv)
       │
       ▼
  ┌─────────────────────────────────┐
  │      Runtime Detection          │
  │                                  │
  │  Vulkan available?               │
  │    └── Yes: Use Vulkan Compute   │
  │                                  │
  │  OpenCL available?               │
  │    └── Yes: Use OpenCL 2.0+      │
  │                                  │
  │  Level Zero available?           │
  │    └── Yes: Use Intel oneAPI L0  │
  │                                  │
  │  chipStar available?             │
  │    └── Yes: Use CUDA/HIP bridge │
  │                                  │
  │  None available?                 │
  │    └── Fall back to CPU SIMD     │
  └─────────────────────────────────┘
```

### Vulkan Compute Dispatch

The Vulkan compute path creates a compute pipeline from the SPIR-V module:

```text
1. VkCreateShaderModule(spv_bytes) → VkShaderModule
2. VkCreateComputePipelines(shader_module, entry_point: "main")
3. VkAllocateDescriptorSets() → bind input/output buffers
4. VkCmdDispatch(group_count_x, group_count_y, group_count_z)
5. VkQueueSubmit() → execute on GPU
6. VkQueueWaitIdle() → synchronize
```

---

## 33.6 Capability Negotiation

The SPIR-V emitter queries the target GPU's capabilities before generating code and selects the appropriate instruction set:

| Capability | Required Extension | Used For |
| :--- | :--- | :--- |
| `Float64` | Core SPIR-V | Double-precision arithmetic |
| `CooperativeMatrixKHR` | `SPV_KHR_cooperative_matrix` | Tensor core matrix ops |
| `AtomicFloat32AddEXT` | `SPV_EXT_shader_atomic_float_add` | Atomic float addition |
| `Int64Atomics` | Core SPIR-V | 64-bit atomic operations |
| `SubgroupBallotKHR` | `SPV_KHR_shader_ballot` | Warp-level voting |
| `PhysicalStorageBuffer` | `SPV_KHR_physical_storage_buffer` | Raw pointer access |

When a required capability is not available, the compiler generates a **fallback implementation** using available instructions, or emits a compile-time diagnostic:

```text
warning[W0803]: cooperative matrix not available on target GPU
  ┌─ src/kernel.agam:8:5
  │
8 │     let C = tile_matmul(tile_a, tile_b);
  │             ^^^^^^^^^^ cooperative matrix not supported
  │
  = note: falling back to software matrix multiplication
  = help: target GPU does not support SPV_KHR_cooperative_matrix
```

---

## 33.7 SPIR-V Validation

All generated SPIR-V modules pass through the **SPIRV-Tools** validator (`spirv-val`) before being submitted to the GPU runtime. Validation catches:

- Malformed instruction encoding
- Type mismatches in operands
- Invalid memory access patterns
- Missing capability declarations
- Incorrect execution mode configurations

This ensures that the Agam compiler never produces invalid GPU code, even for edge-case kernel patterns.
