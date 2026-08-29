# Agam Virtual Machine & JIT Compiler Specification

> **Document Status:** Active Standard  
> **Crates:** `agam_mir`, `agam_jit`, `agam_profile`, `agam_debug`  
> **Test Suite:** `agam_test::unit_passes`, `agam_test::opt_semantics`, `agam_test::perf_speed`

---

## 1. Executive Summary

Agam uses a **Register-Based SSA Intermediate Representation (MIR)** capable of direct in-memory JIT compilation via Cranelift and full native object generation via LLVM.

```
                           Agam MIR Module (SSA CFG)
                                       │
                                       ▼
                       ┌───────────────────────────────┐
                       │   MIR Optimization Pipeline   │
                       │  - SSA Constant Propagation   │
                       │  - Dead Code Elimination      │
                       │  - Function Devirtualization  │
                       │  - Loop Unrolling & SROA      │
                       └───────────────┬───────────────┘
                                       │
                ┌──────────────────────┴──────────────────────┐
                ▼                                             ▼
┌──────────────────────────────┐              ┌───────────────────────────────┐
│     JIT Execution Engine     │              │    Runtime Profiler Engine    │
│          (agam_jit)          │              │        (agam_profile)         │
│  - Cranelift SSA Translation │              │  - Hotspot call counters      │
│  - Dynamic Memory Relocation │              │  - Argument shape feedback    │
│  - Native Calling Conv ABI   │              │  - Specialization hints       │
│  - Direct In-Memory Call     │              │  - Adaptive JIT tiering       │
└───────────────┬──────────────┘              └───────────────┬───────────────┘
                │                                             │
                └──────────────────────┬──────────────────────┘
                                       │
                                       ▼
                         Native CPU Native Code Cache
```

---

## 2. SSA Intermediate Representation (`agam_mir`)

### 2.1 BasicBlock Control Flow Graph
- **Virtual Registers:** Strongly-typed infinite virtual registers with Single Static Assignment invariants.
- **Instructions:** `Assign`, `BinaryOp`, `UnaryOp`, `Load`, `Store`, `GetElementPtr`, `Call`, `Intrinsic`, `Cast`.
- **Terminators:** `Return`, `Branch`, `Switch`, `Yield`, `Resume`, `Unreachable`.

### 2.2 Optimization Passes
- **Constant Folding:** Evaluates constant mathematical and bitwise expressions during compilation.
- **Dead Code Elimination (DCE):** Eliminates unused SSA registers and unreachable basic blocks.
- **Inlining:** Expands small leaf and method calls directly into the caller basic block.
- **Loop Unrolling:** Unrolls fixed-iteration counting loops to minimize branch prediction overhead.

---

## 3. Cranelift JIT Engine (`agam_jit`)

- **In-Memory Translation:** Lowers Agam MIR BasicBlocks directly to Cranelift CLIF IR.
- **Native Relocation:** Dynamically resolves symbols and allocates executable memory pages.
- **Direct Calling Interface:**
  ```rust
  let compiled = CompiledJitModule::compile(&mir, JitOptions::default())?;
  let result = compiled.run_function("compute", &[JitValue::I32(100)])?;
  ```

---

## 4. Adaptive Profiler (`agam_profile`)

- **Execution Counters:** Tracks call frequencies and hot loop cycles.
- **Argument Shape Sampling:** Records argument type stability at polymorphic call sites.
- **Specialization Payoff Heuristics:** Guides function cloning for hot monomorphic instances.
