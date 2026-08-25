# Agam Runtime Systems & Platform Abstraction Specification

> **Document Status:** Active Standard  
> **Crates:** `agam_runtime`, `agam_std`, `agam_ffi`  
> **Test Suite:** `agam_runtime::tests` (57 tests)

---

## 1. Executive Summary

The Agam runtime environment provides low-level operating system abstraction, memory management, execution sandboxing, hardware introspection, portable SIMD vectorization, and algebraic effect handlers.

```
                           Agam Compiled Binary / JIT
                                       │
                                       ▼
                       ┌───────────────────────────────┐
                       │     Runtime Services API      │
                       │  - Memory & ARC Subsystem     │
                       │  - Algebraic Effects Dispatch │
                       │  - Coroutine Work-Stealing    │
                       └───────────────┬───────────────┘
                                       │
                ┌──────────────────────┼──────────────────────┐
                ▼                      ▼                      ▼
┌──────────────────────────────┐ ┌───────────┐ ┌──────────────────────────────┐
│   Platform Abstraction PAL   │ │  Sandbox  │ │   Hardware Introspection     │
│  - Virtual Memory / Arenas   │ │ Isolation │ │  - Cache hierarchy detection │
│  - Native Thread Pool        │ │ (JobObj / │ │  - SIMD auto-dispatch (AVX)  │
│  - Non-Blocking Socket / I/O │ │  prctl)   │ │  - Optimal tile computation  │
└───────────────┬──────────────┘ └─────┬─────┘ └──────────────┬───────────────┘
                │                      │                      │
                └──────────────────────┼──────────────────────┘
                                       │
                                       ▼
                         Operating System Kernel & HW
```

---

## 2. Core Runtime Subsystems

### 2.1 OS Execution Sandboxing (`sandbox.rs`)
- **Windows:** Win32 Job Objects with memory quotas, CPU time ceilings, and `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`.
- **Linux:** Process isolation via `prctl` and `setrlimit` resource bounds.
- **Watchdog Timer:** Independent background watchdog thread aborting hung evaluations.

### 2.2 Hardware Introspection (`hwinfo.rs`)
- **Cache Topology:** Detects L1/L2/L3 cache sizes and cache-line boundaries.
- **SIMD Tiers:** Auto-detects SSE4.2, AVX2, AVX-512, and ARM NEON.
- **Optimal Tiling:**
  - `optimal_tile_size(bytes)`: Computes cache-resident matrix tiles.
  - `optimal_chunk_size()`: Computes multi-threaded chunk sizes.

### 2.3 Portable SIMD Vectorization (`simd.rs`)
- High-level portable vector math (`simd_add`, `simd_mul`, `simd_fma`, `simd_dot`, `simd_norm`).
- Tiled matrix multiplications (`2x2`, `3x3`, `NxM`) operating over aligned arrays.

### 2.4 Algebraic Effect Handlers (`effects.rs`)
- Dynamic thread-local effect registry mapping effect types to active handler closures.
- Resumable continuation frames enabling customizable asynchronous control flow, generators, and test mocking.
