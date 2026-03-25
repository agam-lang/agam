<div align="center">
  <h1>Agam Language 🚀</h1>
  <p><b>A natively compiled omni-language unifying Python's simplicity with C++'s raw hardware control, Rust's memory safety, and native mathematical primitives natively designed for AI and Data Science.</b></p>
</div>

---

## 🌟 The Vision

Programming today forces a painful tradeoff: you write prototype models in Python (slow, high-level, interpreted), but when you need scale or deployment, you rewrite it in C++ or Rust (complex, low-level, strict). 

**Agam** is designed to end this dichotomy. It provides the fluid, dynamic ergonomics of Python while seamlessly scaling down to the bare-metal SIMD/GPU cache-aligned performance of C++ and the fearless concurrency of Rust. Above all, it treats **mathematics, machine learning, and hardware** as fundamental primitives, not glued-on libraries.

---

## ✨ Features Built from the Basics

### 1. Dual-Mode Typing & Modern Compilation
* **`dyn` vs `static` Modes**: Start with dynamic typing (`var x: dyn = 10`) for fast prototyping, and progressively harden your code with strict types (`let x: i32 = 10`) for production.
* **Hindley-Milner Inference**: A powerful semantic engine infers your types perfectly without excessive annotations.
* **Const Evaluation (Comptime)**: Run code at compile-time to guarantee zero-cost abstractions at runtime.

### 2. Hybrid Memory Management
* **Automatic Reference Counting (ARC)**: By default, memory is managed via lightweight, deterministic `AgamArc<T>` (unlike garbage collection stalls).
* **Strict Lifetimes Opt-in**: Need zero-allocation inner loops? Use the `strict { ... }` block to enforce Rust-like borrow-checking and strict lifetimes natively, disabling ARC entirely for that block.

### 3. First-Class Differentiable Programming
Forget large, complex autodiff frameworks (PyTorch/JAX). Agam has differentiation built into the compiler:
* **`grad` and `backward` keywords**: Built directly into the AST and generated natively.
* **Dual-Mode AD**: Forward-mode via dual numbers and Reverse-mode via compiler-tracked `GradTape`.
* Native `tensor` type optimized for the underlying hardware.

### 4. Hardware-Aware Execution
Agam knows the machine it's running on:
* **`hwinfo` Runtime**: Automatically detects CPU topology, L1/L2/L3 cache sizes, and SIMD capabilities (SSE2, AVX-512, NEON).
* **`#[align(L1_Cache)]`**: Native compiler attributes that automatically tile matrices and align memory exactly to the user's processor cache lines.
* **`#[dispatch(SIMD)]`**: Automatic vectorization of operations directly in the backend.

### 5. Scientific Standard Library (`agam_std`)
Built from scratch to be blazingly fast and mathematical natively:
* **`ndarray` & `dataframe`**: Native, columnar, zero-copy data structures for Pandas/NumPy-like ergonomics.
* **`math` & `linalg`**: Native FFT, Simpson/Gauss integration, LU/QR decompositions, eigenvalue solvers.
* **`ml`**: Native primitive operations for neural networks, including Loss functions, GELU/Swish activations, Batch Norm, and KNN.
* **`units`**: Compile-time SI unit dimensional analysis (preventing catastrophic rocket crashes by multiplying `m/s` by `kg` safely).

### 6. Algebraic Effects System
Callback hell and uncolored async functions are solved via modern Algebraic Effects:
* `effect`, `handle`, and `resume` keywords allow separating side effects (like IO, state, async) from pure logic. 
* The compiler uses Continuation-Passing Style (CPS) to lower these efficiently.

### 7. Refinement Types & Compile-Time Proofs (SMT)
Agam integrates an SMT-LIB2 solver (Z3) directly into the type-checker.
* Define types like `{v: i32 | v != 0}`.
* The compiler **mathematically proves** there are no array-out-of-bounds or divide-by-zero errors *before* the code ever runs.

---

## 🥊 How Agam Compares to Other Languages

| Feature | Agam | Python | C++ | Rust | Mojo | Julia |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| **Learning Curve** | Easy -> Gradual | Easiest | Steepest | Steep | Easy | Medium |
| **Memory Management**| ARC / Strict Borrowing | GC | Manual / RAII | Borrow Checker | ARC / Manual | GC |
| **Execution** | Native / AOT | Interpreted | Native AOT | Native AOT | Native AOT | JIT |
| **Null Safety** | ✅ Yes (Option) | ❌ No | ❌ No (Pointers) | ✅ Yes | ✅ Yes | ❌ No |
| **Autodiff (Native)**| ✅ Yes (Keywords)| ❌ (Libraries) | ❌ (Libraries) | ❌ (Libraries) | ❌ (Libraries)| ❌ (Libraries) |
| **Compile-Time Proofs**| ✅ SMT Refinement | ❌ No | ❌ No | ❌ No | ❌ No | ❌ No |
| **Algebraic Effects**| ✅ Yes | ❌ No | ❌ No | ❌ No | ❌ No | ❌ No |

### The Agam Advantage
* **Vs Python**: 100x faster, true multi-threading (no GIL), compile-time safety, but with the same clean, block-oriented readable syntax.
* **Vs Rust**: No fighting the borrow checker while prototyping (thanks to ARC and `dyn`). You opt-in to strict lifetimes only when optimizing the critical paths.
* **Vs C++**: Memory safe by default. Modern modularity. No confusing header files or ancient CMake build systems.
* **Vs Mojo**: Agam is not beholden to Python's legacy C-API overhead. It brings entirely new primitives (Effects, Refinement Types, Hardware abstractions) that Mojo lacks.

---

## 🔮 Future Scopes & Paradigm Shifts (Phase 11+)

Agam isn't just catching up to modern languages; it is looking a decade ahead.

1. **Capability-Based Security (Phase 11)**: Replacing traditional OS permissions. Functions will declare `cap: FileRead`, restricting them at a compiler level from making arbitrary networks or file calls. The ultimate defense against supply-chain attacks.
2. **Content-Addressable Code Base (Phase 12)**: The end of `semver` dependency hell. Modules will be resolved via BLAKE3 AST hashing on a decentralized network. You can rename functions across the internet without breaking downstream code.
3. **Quantum-Ready Primitives (Phase 13)**: The standard library will include the `Qubit` type and gates (H, X, Z, CNOT), pairing with classical architectures seamlessly.
4. **E-Graph Compiler Optimization (Phase 14)**: Reinforcement Learning (RL) agents will explore an equivalence graph to find the mathematically cheapest way to execute a program on a given CPU topology.
5. **Zero-Knowledge Proof Primitives (Phase 15)**: `#[zkp]` annotations will compile Agam code directly into zk-SNARK circuits for cryptographic verifiability on blockchain networks.

---

*Agam: The language that scales from the drawing board to the datacenter.*
