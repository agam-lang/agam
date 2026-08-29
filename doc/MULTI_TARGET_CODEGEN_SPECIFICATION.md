# Agam Multi-Target Code Generation Specification

> **Specification Status:** Active Standard  
> **Crates:** `agam_codegen`, `agam_jit`, `agam_driver`  
> **Test Suites:** `agam_test::llvm_output`, `agam_test::gpu_output`, `agam_test::c_output`, `agam_test::toolchain_output`

---

## 1. Overview

Agam compiles from a unified Mid-Level Intermediate Representation (MIR) to multiple native target representations.

```
                     ┌──────────────────┐
                     │     Agam MIR     │
                     │ (SSA BasicBlocks)│
                     └─────────┬────────┘
                               │
       ┌───────────────────────┼────────────────────────┬──────────────────────┐
       ▼                       ▼                        ▼                      ▼
┌───────────────┐      ┌───────────────┐        ┌───────────────┐      ┌───────────────┐
│   C Emitter   │      │  LLVM Emitter │        │  GPU Emitter  │      │  JIT Backend  │
│  (ANSI C11)   │      │   (LLVM IR)   │        │ (NVPTX/CUDA)  │      │  (Cranelift)  │
└───────┬───────┘      └───────┬───────┘        └───────┬───────┘      └───────┬───────┘
        ▼                      ▼                        ▼                      ▼
┌───────────────┐      ┌───────────────┐        ┌───────────────┐      ┌───────────────┐
│ Clang / MSVC  │      │   LLC / Opt   │        │  NVCC / CUDA  │      │ Direct Native │
│ Bare-Metal C  │      │ Native Object │        │ GPU Kernels   │      │ In-Memory Run │
└───────────────┘      └───────────────┘        └───────────────┘      └───────────────┘
```

---

## 2. Target Profiles & Code Generation Backends

### 2.1 ANSI C11 Emitter (`agam_codegen::c`)
- **Headers & Types:** Standard headers (`<stdint.h>`, `<stdbool.h>`, `<stdlib.h>`), tagged union `AgamEnum` layouts.
- **Embedded Mode (`@target.iot`):** Generates `#define AGAM_NO_HEAP 1` and `#define AGAM_TARGET_IOT 1`, eliminating dynamic allocations.
- **Algebraic Effect Runtime:** Static effect dispatch tables and continuation frame structs.

### 2.2 LLVM IR Emitter (`agam_codegen::llvm`)
- **Typed SSA Values:** Direct mapping to `i1..i512`, `float`, `double`, `[N x T]`, and struct aggregate types.
- **Memory Instructions:** Strict `alloca`, `load`, `store`, and typed `getelementptr inbounds`.
- **Target Datalayouts & Metadata:** Emits target datalayout strings, target triples (`x86_64`, `aarch64`, `riscv64`, `wasm32`), and `@target.hpc` / `@target.iot` named metadata.
- **Call Cache Integration:** Automatically emits memoized call cache wrappers (`@__agam_cached_...`) for pure functions.

### 2.3 GPU & PTX Emitter (`agam_codegen::gpu`)
- **Target Triple:** `nvptx64-nvidia-cuda`.
- **Kernel Declarations:** `define ptx_kernel void @...` with `addrspace(3)` shared memory arrays.
- **Special Register Intrinsics:** `@llvm.nvvm.read.ptx.sreg.tid.x`, `@llvm.nvvm.read.ptx.sreg.ctaid.x`, etc.
- **Host Linkage:** Declarations for `cudaMalloc`, `cudaMemcpy`, `cudaLaunchKernel`, and `cudaFree`.
- **Constraint Enforcement:** Prohibits heap allocations, strings, recursion, and dynamic effects inside kernel bodies.

### 2.4 JIT Engine (`agam_jit`)
- In-memory SSA compilation using Cranelift.
- Instant execution for REPLs, interactive scripting, and live test harness runners.

---

## 3. Toolchain Command Orchestration

The compiler driver (`agam_driver::toolchain`) auto-detects installed toolchains and synthesizes optimized command lines:
- **Clang / Clang++:** `-O0..-O3`, `-std=c11` / `-std=c++20`, `-target <triple>`.
- **MSVC (`cl.exe`):** `/O2`, `/std:c11`, `/W4`.
- **LLVM (`llc` / `opt`):** `-filetype=obj`, `-mcpu=native`, `-O3`.
