# Architectural Comparison Matrix: Agam vs. Major Compilers

> **Scope**: Comparative architectural analysis of **Agam** vs. **Rust (`rustc`)**, **Mojo**, **Triton**, **Zig**, and **Python (`CPython`)**.

---

## 1. Feature & Capability Comparison Table

| Architecture Dimension | **Agam** | **Rust (`rustc`)** | **Mojo** | **Triton** | **Zig** | **Python (`CPython`)** |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Type System** | Static interned `TypeStore`, Enums, Generics, Option/Result | Static linear/borrow-checked type system | Static MLIR-backed struct/type system | Dynamic Python frontend $\rightarrow$ Static MLIR | Static `comptime` type system | Dynamic object model |
| **Effects System** | Native Effect Perform & Handler runtime contract | No native effects (via async/traits) | No native effects | No native effects | No native effects | No native effects |
| **Backend Code Generation** | **4-in-1**: Native LLVM IR, Universal GPU Emitter (NVPTX / AMDGPU / SPIR-V / Metal), C Emitter, Cranelift JIT | LLVM IR (Cranelift opt) | MLIR $\rightarrow$ LLVM IR | MLIR $\rightarrow$ LLVM/NVPTX | Custom C/LLVM | Bytecode Interpreter |
| **GPU / HPC Lowering** | Direct Universal GPU Emitter (`addrspace(3)` shared alloc, barrier intrinsics, multi-vendor target adapters) | External CUDA / nvcc via build scripts | MLIR GPU Dialect | Python DSL $\rightarrow$ NVPTX | External C/CUDA | PyTorch/CUDA wrappers |
| **IR Compile Throughput** | **300–374 MB/s** (pre-allocated buffer codegen) | Medium (slow LLVM codegen) | Medium (MLIR optimization pipeline) | Fast (domain specific) | Fast (custom C codegen) | N/A (Interpreted) |
| **JIT & Call Caching** | Adaptive pure call cache, JIT specialization guards | No JIT (AoT only) | AoT / JIT via MLIR | JIT per kernel launch | AoT only | PyPy/Numba JIT |
| **Agentic / MCP Native** | Built-in `agamc mcp serve`, SARIF streaming, telemetry loops | External cargo tools | External | External | External | External |

---

## 2. Deep-Dive Architectural Differences

### 2.1 Agam vs. Rust (`rustc`)
- **Type System & Interning**: Agam uses an interned `$O(1)$` lookup arena `TypeStore` with hash-deduplicated `TypeId` references for all resolved types (`Option<T>`, `Result<T, E>`, tuples, arrays), avoiding quadratic type-check overhead on large projects.
- **Universal GPU Integration**: Unlike Rust which relies on `nvcc` or external vendor crates for GPU execution, Agam features a first-class **Universal GPU Emitter** built directly into `agam_codegen` (supporting NVPTX, AMDGPU, SPIR-V, and Metal target adapters), generating target assembly, `addrspace(3)` shared allocations, and native driver launching.
- **Effects Model**: Agam features built-in algebraic effects (`Op::EffectPerform`, `Op::HandleWith`) lowering natively through MIR.

### 2.2 Agam vs. Mojo / Triton
- **Domain Scope**: Mojo and Triton focus exclusively on ML tensor kernels via MLIR. Agam provides a complete general-purpose systems language pipeline (AST $\rightarrow$ HIR $\rightarrow$ MIR $\rightarrow$ Codegen) that compiles general applications, web services, CLI tools, AND GPU kernels.
- **Dependency Weight**: Triton requires a heavy Python runtime and MLIR dependency chain. Agam compiles to zero-dependency standalone binaries via its native C and LLVM emitters.

### 2.3 Agam vs. Zig
- **JIT & Adaptive Profiling**: Zig is strictly Ahead-of-Time (AoT). Agam includes both AoT LLVM/C code generation and a lightweight **Cranelift JIT runtime** (`agam_jit`) with adaptive call caching and profile-guided specialization.
- **Agent Protocol Integration**: Agam includes built-in AI agent protocol support (`agamc mcp serve`) for direct interaction with LLM coding tools.

---

## 3. Performance Summary Matrix

| Metric | Agam Performance | Baseline Reference |
| :--- | :--- | :--- |
| **Single Kernel Codegen Latency** | **103.4 µs** / iter | 1,000 instrs / 1,000 iters |
| **Batch Codegen Throughput** | **374 MB/s** IR generation | 100 kernels $\times$ 200 instrs |
| **Type Comparison Complexity** | **$O(1)$** Interned TypeID | Structural comparison $O(N)$ |
| **Workspace Test Suite Verification** | **100% Pass** across 27 crates | All backends (LLVM/NVPTX/C/JIT) |
