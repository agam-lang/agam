# Stream 1: Systems & Core Construction Stream

## Overview

Stream 1 governs the active daily construction of Agam compiler features across Technical Tiers T0–T6 (Foundation, DX, Security, Platform, Optimization, AI-Native, Frontier).

## Construction Rules & Architectural Principles

1. **Pipeline Discipline:**
   - Every language feature follows strict lowering: AST → HIR → MIR → Codegen (LLVM / NVPTX / C / JIT).
   - Never skip MIR lowering or write ad-hoc backend bypasses.

2. **Monomorphization & Type Safety:**
   - Generic functions and sum types (`Option<T>`, `Result<T, E>`) monomorphize cleanly without runtime overhead.

3. **GPU & Tile-IR Lowering (MiniTriton-inspired):**
   - `@gpu(...)` kernels lower through `Op::GpuKernelLaunch`, `Op::GpuSharedAlloc`, and address-space qualified pointers (`addrspace(1..5)`).

4. **Omni-Targeting & Effect Guardrails:**
   - `@target.iot` functions reject heap allocations and algebraic effects compile-time.
