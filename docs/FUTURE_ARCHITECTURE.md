# Agam Long-Term Goal Architecture (Design Target)

> **Document Type**: Future Architectural Specification & Design Target  
> **Status**: Design Blueprint (Not a Status Report)  
> **Near-Term Priorities**: See [ROADMAP.md](ROADMAP.md)  
> **Active Milestone Checklists**: See [.agent/specs/active/details/STAGE-00-driver-modularization-and-hardening.md](.agent/specs/active/details/STAGE-00-driver-modularization-and-hardening.md) through `STAGE-07`  

---

## 1. Purpose Statement

This document defines the long-term architectural target for the Agam compiler, intermediate representations, native datatype universe, and heterogeneous computing backend strategy. It represents the target shape of the system to guide future engineering phases. For current compiler capabilities and verified features on `origin/main`, refer to [README.md](README.md).

---

## 2. The 7-Layer Progressive Lowering Pipeline

```
                                  PROGRESSIVE IR TARGET PIPELINE
┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ Layer 1: Pāṇinian AST & SEMA   │ Syntax, Pratt Panic Recovery, Type Sandhi Lattice, Refinements        │
├────────────────────────────────┼───────────────────────────────────────────────────────────────────────┤
│ Layer 2: High-Level IR (HIR)   │ Delimited Continuations (Stack-Promoted CPS), Baur–Strassen Autodiff  │
├────────────────────────────────┼───────────────────────────────────────────────────────────────────────┤
│ Layer 3: Tensor Polyhedral IR  │ N-Dimensional Contractions, 64-Byte Affine Loop Tiling, Kernel Fusion │
├────────────────────────────────┼───────────────────────────────────────────────────────────────────────┤
│ Layer 4: Mid-Level SSA (MIR)   │ Lengauer–Tarjan Dominators, Interprocedural Static ARC Escape Elision │
├────────────────────────────────┼───────────────────────────────────────────────────────────────────────┤
│ Layer 5: E-Graph Saturation    │ Congruence Closure Equality Saturation via `egg`                      │
├────────────────────────────────┼───────────────────────────────────────────────────────────────────────┤
│ Layer 6: Low-Level IR (LIR)    │ Hardware Address Spaces (0..4), George–Appel Iterated RegAlloc        │
├────────────────────────────────┼───────────────────────────────────────────────────────────────────────┤
│ Layer 7: Heterogeneous Hub     │ In-Process LLVM, Cranelift JIT, NVPTX (CUDA), ROCm, SPIR-V, NPU      │
└────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

### Pipeline Layer Status & Implementation Reality

| Pipeline Layer | Architectural Role | Current Status | Existing Codebase Reality |
|:---|:---|:---:|:---|
| **Layer 1: AST & SEMA** | Pratt parsing, dual `@lang.base`/`@lang.advance` profiles, type checking | 🟡 `[Partially Implemented]` | Working lexer, Pratt parser, and SEMA in `crates/frontend/agam_parser/` and `agam_sema/`. Error recovery synchronization is not yet implemented (Stage 0). |
| **Layer 2: High-Level IR (HIR)** | Desugaring, pattern lowering, algebraic effects, autodiff | 🟡 `[Partially Implemented]` | Structural AST-to-HIR lowering exists in `crates/middle/agam_hir/`. Effect handlers and reverse-mode autodiff are experimental/unwired stubs. |
| **Layer 3: Tensor Polyhedral IR (T-IR)** | $N$-dimensional iteration spaces, loop tiling, affine kernel fusion | 🟡 `[Partially Implemented]` | Basic polyhedral pass scaffold exists in `crates/middle/agam_mir/src/opt/polyhedral.rs` (13.4KB), but affine scheduling and loop tiling are not active in default pipelines. |
| **Layer 4: Mid-Level SSA (MIR)** | Target-independent SSA CFG, GVN, SCCP, escape analysis | 🟢 `[Implemented]` | Full SSA CFG lowering in `crates/middle/agam_mir/` with GVN, constant fold, and inliner. Escape analysis exists in `src/opt/escape.rs` but is not wired into the main fixed-point loop. |
| **Layer 5: E-Graph Saturation** | Phase-ordering-free algebraic optimization | 🟡 `[Partially Implemented]` | Scaffolding exists in `crates/middle/agam_mir/src/opt/egraph/`. Will adopt the `egg` crate per [ADOPTED_DEPENDENCIES.md](ADOPTED_DEPENDENCIES.md). |
| **Layer 6: Low-Level IR (LIR)** | Hardware address space qualification, virtual register allocation | ⚪ `[Proposed / Not Started]` | Design target only. Currently, MIR lowers directly to Cranelift JIT IR or textual LLVM IR. |
| **Layer 7: Backend Targets** | In-process LLVM AOT, Cranelift JIT, NVPTX, ROCm, SPIR-V | 🟡 `[Partially Implemented]` | Cranelift JIT (`agam_jit`) and Textual LLVM AOT (`agam_codegen`) are functional. NVPTX text emission exists in `gpu_emitter.rs` (1,594 lines). In-process LLVM C-API, ROCm, SPIR-V, and NPU are proposed. |

---

## 3. Adopt vs. Build Architectural Boundary

Agam strictly avoids hand-rolling mature, peer-reviewed computer science infrastructure. For full details on third-party library adoption policies, see [ADOPTED_DEPENDENCIES.md](ADOPTED_DEPENDENCIES.md).

- **E-Graph Saturation**: Adopt **`egg`** crate.
- **SMT Refinements**: Adopt **`z3` / `z3-sys`** crate.
- **Polyhedral Loop Tiling**: Adopt **`isl-rs` / LLVM Polly**.
- **Post-Quantum Cryptography**: Adopt audited crates (**`ml-kem`**, **`pqcrypto`**, **`sha2`**, **`blake3`**, **`aes-gcm`**).
- **In-Process JIT**: Adopt **`cranelift-codegen`**.
- **In-Process AOT**: Adopt **LLVM 18+ C-API**.

---

## 4. Native Datatypes Roadmap

Status definitions:
- 🟢 `[Implemented & Tested]`: Native type is fully supported across frontend, MIR, and codegen.
- 🟡 `[Partially Implemented]`: Type label, test, or prototype struct exists, but is not a full first-class language type.
- ⚪ `[Proposed / Not Started]`: Design specification only; no language-level implementation exists today.

| Proposed Native Type | Category / Purpose | Status | Current Codebase Reality |
|:---|:---|:---:|:---|
| `i8`..`i64`, `u8`..`u64` | Standard Scalar Integers | 🟢 `[Implemented]` | Native primitives supported across all crates. |
| `f32`, `f64` | IEEE 754 Floating Point | 🟢 `[Implemented]` | Native primitives supported across all crates. |
| `bool`, `str` | Primitives & String Slices | 🟢 `[Implemented]` | Native primitives supported across all crates. |
| `quaternion` | 3D Rigid Body Rotations | 🟢 `[Implemented]` | Fully implemented with unit tests in `crates/runtime/agam_std/src/complex.rs`. |
| `bf16` | Bfloat16 AI Float | 🟡 `[Partially Implemented]` | Exists only as a label in `NpuPrecision::Bf16 => "__bf16"` (not a first-class language type). |
| `strided_view` | Zero-Copy Tensor Slicing | 🟡 `[Partially Implemented]` | Appears as test function `test_strided_view_access`, but no public type/API exists today. |
| `f8e4m3`, `f8e5m2` | OCP Microscaling FP8 Formats | ⚪ `[Proposed]` | Target type for AI activation and gradient matrix math. |
| `nf4` | NormalFloat4 for 4-bit QLoRA | ⚪ `[Proposed]` | Target type for low-power edge quantized weights. |
| `f128` | Quadruple Precision Float | ⚪ `[Proposed]` | Target type for scientific and aerospace simulations. |
| `vec<T, N>` | First-Class SIMD Register Type | ⚪ `[Proposed]` | Target type for explicit vector math (`vec8f32`, `vec64u8`). |
| `tile<T, M, N>` | Hardware 2D Matrix Register | ⚪ `[Proposed]` | Target type for NVIDIA Tensor Core / Intel AMX matrix tiles. |
| `u1`..`u1024` | Arbitrary-Width Integers | ⚪ `[Proposed]` | Target type for bitfields (`u1`), audio (`u24`), crypto (`u256`). |
| `q16.16`, `q8.24` | Fixed-Point Deterministic DSP | ⚪ `[Proposed]` | Target type for motor control and embedded audio DSP. |
| `dec64`, `dec128` | Exact Decimal Floating Point | ⚪ `[Proposed]` | Target type for IEEE 754-2008 financial accounting. |
| `span<T>` | Non-Owning Contiguous Slice | ⚪ `[Proposed]` | Target type for 64-byte aligned memory spans. |
| `owned<T>` | Affine Single-Owner Pointer | ⚪ `[Proposed]` | Target type for move-only zero-runtime resources. |
| `complex<T>` (Generic)| Generic Complex Numbers | ⚪ `[Proposed]` | Specialized `complex64` exists; generic parameterization proposed. |
| `gf<2^n>` | Galois Finite Field | ⚪ `[Proposed]` | Target type for hardware PCLMULQDQ / PMULL crypto primitives. |

---

## 5. Explicit Non-Goals for the Current Year (Scope Pruning)

To ensure high engineering reliability, the following items are **explicitly deferred to Stage 8+ (Future Research)** and will not be built in Year One:

```
                          EXPLICIT YEAR-ONE SCOPE BOUNDARY
┌──────────────────────────────────────────────────┬──────────────────────────────────────────────────┐
│ 🟢 IN SCOPE FOR YEAR ONE (Core Production Focus) │ 🔴 EXPLICITLY DEFERRED TO STAGE 8+ (Not Now)     │
├──────────────────────────────────────────────────┼──────────────────────────────────────────────────┤
│ • Cranelift In-Process JIT (Dev / Scripting)     │ • GCC / libgccjit Backend (Redundant with LLVM)  │
│ • LLVM AOT (Self-contained, in-memory bindings)  │ • AMD ROCm & Vulkan SPIR-V GPU Targets           │
│ • NVIDIA NVPTX64 (CUDA Kernel Execution)         │ • NPU / TPU Systolic Array Custom Lowerings      │
│ • Swift-style ARC + Static Escape Elision        │ • JIT-to-MIR Speculative Tiering (OSR / Deopt)   │
│ • Opt-in affine `strict {}` ownership            │ • Complex E-Graph / Polyhedral hand-rolled passes│
│ • Pratt parser panic-mode token recovery         │ • 15 non-core datatypes (f128, u1024, gf<2^n>)   │
│ • Zero-panic compiler core (Result<T, Error>)   │ • Distributed package registry network           │
└──────────────────────────────────────────────────┴──────────────────────────────────────────────────┘
```

---

## 6. Verification Requirement

Every capability claimed as 🟢 or 🟡 must be verifiable by checking the repository directly. For example:
- `Op::Phi` and indexed memory: `crates/backends/agam_codegen/src/llvm_emitter.rs`
- C-ABI runtime exports: `crates/runtime/agam_runtime/src/export.rs`
- NVPTX64 kernel emission: `crates/backends/agam_codegen/src/gpu_emitter.rs`
- Quaternion implementation: `crates/runtime/agam_std/src/complex.rs`
