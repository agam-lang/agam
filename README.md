<div align="center">
  <h1>Agam: The Omni-Language Compiler Project 🚀</h1>
  <p><b>From the absolute basics of compilers and tensors to advanced SMT mathematical proofs.</b></p>
</div>

---

## 📖 Introduction: Why Build a New Language?

For AI-agent workflow rules, operating constraints, and phase-completion policy, read [`.agent/agent.md`](./.agent/agent.md) before making repository changes.

In software development today, we face a **"two-language problem"**:
1. **Python** is incredibly easy to read and perfect for experimenting (Data Science, AI), but it runs slowly because it is interpreted.
2. **C++ or Rust** run at blazing native hardware speeds and manage memory safely, but they have steep learning curves and strict rules that slow down rapid prototyping.

**Agam** is a new programming language project that solves this. It aims to have the simple readability of Python, but the raw hardware speed of C++ and the memory safety of Rust. Most importantly, it treats **Mathematics and Artificial Intelligence** not as external libraries (like PyTorch or TensorFlow), but as fundamental building blocks of the language itself.

This repository builds the entire Agam compiler and runtime from the ground up. Below is an educational walkthrough of every module we have built, explaining the theoretical basics of *what* it is, and *how* we built it in Agam.

---

## 💻 How to Program in Agam (Quick Start Guide)

Agam's syntax is heavily inspired by Rust's safety and Python's readability. Here are the core features currently implemented in the compiler:

### 1. Variables and Mutability
Bindings are mutable by default so everyday code stays lightweight. Use `const` when you want an explicit immutable value. Legacy `let mut` syntax is still accepted for compatibility, but it is no longer required.
```rust
let name = "Agam";       // Type inferred as String, mutable by default
let x: i32 = 10;         // Explicit type declaration
const version = "0.1";   // Explicitly immutable
let counter = 0;         // Plain bindings can be reassigned
counter = counter + 1;
```

### 2. Control Flow
Standard `if/else` and `while` loop syntax. 
```rust
if counter > 10 {
    print_str("Over ten!");
} else {
    print_str("Under ten!");
}

let i = 0;
while i < 5 {
    print_int(i);
    i = i + 1;
}
```

### 3. Functions
Functions use the `fn` keyword and explicitly declare argument and return types.
```rust
fn calculate_sum(limit: i32) -> i32 {
    let total = 0;
    let i = 0;
    while i < limit {
        total = total + i;
        i = i + 1;
    }
    return total;
}
```

### 4. Advanced ML Data Structures
Agam has native keywords and syntax for Machine Learning data. You don't need to import external libraries for core ML ops.
```rust
// Instantiating a DataFrame and NdArray natively
let df = dataframe.DataFrame { id: 1 };
let inputs = ndarray.NdArray { id: 2 };

// Functional/Closure syntax for native data transformations
let evens = agam_filter(df, |row| { row.val % 2 == 0 });

// Executing Neural Network primitives
let model = agam_dense_layer(inputs, 128);
let loss = agam_mse_loss(model, targets);
```

### 5. Compiling and Running
Use the built-in compiler driver `agam_driver` to build and execute your scripts natively:
```bash
# Fast current-path build: resolves the best available backend automatically
agamc build my_script.agam --fast

# Run with the same fast preset
agamc run my_script.agam --fast

# Keep source formatting clean during development
agamc fmt --check .

# Or build with an explicit optimization level/backend
agamc build my_script.agam --backend llvm -O 3
./my_script.exe
```

---

## 🛠️ Phase 1 & 2: The Basics of Reading Code
### What is a Compiler and a Lexer?
A **Compiler** is a program that translates human-readable code into 1s and 0s that a computer processor can execute. 
Before it can translate, it needs to read the text. A **Lexer** (or Tokenizer) reads the raw text file character by character and groups them into meaningful chunks called **Tokens** (like finding the words in a sentence).

### What We Built:
* **`agam_lexer`**: We built a custom UTF-8 scanner that reads Agam code and converts it into `TokenKind` enums (e.g., recognizing `let`, `+`, `"hello"`, or `123`).
* **Indentation Tracking**: Like Python, Agam uses indentation (tabs/spaces) to figure out which code belongs together. Our lexer tracks this and secretly inserts virtual `{` and `}` braces for the next phase.

---

## 🌳 Phase 3: Understanding the Grammar
### What is an AST and a Parser?
Once we have the "words" (Tokens), we need to understand the "sentence structure." 
A **Parser** organizes the tokens into a tree-like data structure called an **AST (Abstract Syntax Tree)**. It enforces the grammar rules of the language.

### What We Built:
* **`agam_ast` & `agam_parser`**: We created AST nodes to represent everything you can write (Functions, Variables, Math).
* **Pratt Parser**: A specific parsing algorithm (Top-Down Operator Precedence) that is incredibly good at understanding complex mathematical formulas safely (knowing that `2 + 3 * 4` means `2 + (3 * 4)`).
* **Dual Typing**: Our parser supports both Python-like dynamic variables (`var x = 10`) and Rust-like strict types (`let x: i32 = 10`).

---

## 🤔 Phase 4: Semantic Analysis (The Brain)
### What is Semantic Analysis and Type Inference?
The parser knows the *grammar*, but the **Semantic Analyzer** checks if the code actually makes *sense*. For example, adding `"apple" + 5` is grammatically correct, but semantically bad.
**Type Inference** is a mathematical algorithm where the compiler acts like a detective, automatically guessing the data types of your variables based on how you use them so you don't have to write them out manually.

### What We Built:
* **`agam_sema`**: We implemented **Hindley-Milner Algorithm W** for perfect type inference.
* **Ownership & Lifetimes**: We built a borrow-checker (like Rust's) that prevents memory from being accessed after it gets deleted, avoiding crashes.
* **Comptime Evaluation**: Agam can execute mathematical code *during the compilation process itself*, resulting in zero delay when the program actually runs.

---

## ⚙️ Phase 5 & 6: Code Generation & Memory Management
### What is Memory Management (GC vs ARC vs Borrowing)?
When programs create variables, they use RAM. 
* *Python/Java* use **Garbage Collection (GC)**: A background program sporadically pauses your app to clean up RAM. 
* *C++* requires **Manual Management**: The programmer frees RAM, which causes 70% of security bugs.
* **ARC (Automatic Reference Counting)**: The program keeps a tally of how many times a variable is used, and deletes it exactly when the count hits zero. No pauses.

### What We Built:
* **Hybrid Memory**: By default, Agam uses deterministic `AgamArc<T>` (ARC) for ease of use. But, if you wrap code in a `strict { }` block, it switches to zero-allocation Rust-like borrow checking for maximum performance.
* **C-Backend**: We transpile our AST into Highly Optimized C Code (`agam_codegen`), which is then compiled to machine code by LLVM/GCC.

---

## 🧠 Phase 7: Artificial Intelligence Basics
### What is a Tensor and Autodiff?
* **Libraries like PyTorch / TensorFlow**: Usually, languages don't understand AI. Python uses external, massive C++ libraries to do math.
* **Tensor**: A mathematical grid of numbers. A 1D Tensor is a List, a 2D Tensor is a Matrix, a 3D Tensor is a Cube of numbers. Neural networks are entirely built by multiplying and modifying Tensors.
* **Autodiff (Automatic Differentiation)**: The mathematical algorithm (Calculus Chain Rule) that allows neural networks to "learn" by calculating gradients (slopes of errors).

### What We Built from Scratch:
* **Native Types**: We integrated `tensor` as a core keyword in the language.
* **Differentiable Programming**: We added `grad` and `backward` keywords directly into the AST. The compiler literally writes the calculus derivatives for you natively using Forward-Mode Dual Numbers and Reverse-Mode Tapes.
* **`agam_std/ml.rs`**: We built native Neural Network primitives without external dependencies: Dense Layers, Loss Functions, GELU activations, and Math transformations.

---

## 🚀 Phase 8: Hardware Optimization
### What is SIMD and Cache?
* **Cache**: Your CPU has ultra-fast local memory (L1/L2/L3 cache). Reading from main RAM is slow. Making sure data perfectly fits in the Cache is crucial for speed.
* **SIMD (Single Instruction, Multiple Data)**: Hardware features (like AVX or NEON) that allow the CPU to do math on 4 or 8 numbers simultaneously in one clock cycle instead of sequentially.

### What We Built:
* **`hwinfo`**: Agam detects your exact processor topology at runtime.
* **Compiler Attributes**: We built `#[align(L1_Cache)]` and `#[dispatch(SIMD)]` tags. Applying these tells Agam to auto-vectorize your math operations and perfectly slice your Tensors so they fit exactly into your specific CPU's L1 cache, achieving C++ speeds.

---

## 🎭 Phase 9: Managing Side Effects
### What are side effects and Async code?
When code talks to the outside world (reads a file, downloads from a network), it is "side-effectful". It takes time, freezing the app. Languages invented `Promises` or `async`/`await` to fix this, but it turns code messy (the "colored function" problem).
**Algebraic Effects** are an advanced computer science concept where you pause a function right where the effect happens, do the chore outside, and resume seamlessly.

### What We Built:
* **`effect`, `handle`, `resume`**: Agam controls side effects natively using Continuation-Passing Style (CPS), keeping your ML scripts looking clean without `async` keyword pollution.

---

## 🛡️ Phase 10: Provable Mathematical Safety
### What is an SMT Solver and Refinement Types?
Even strictly typed languages let you crash if you divide by a variable that happens to be zero at runtime. 
An **SMT Solver** (Satisfiability Modulo Theories) is a mathematical engine that mathematically proves if a physical state in an equation is possible.

### What We Built:
* **`agam_smt`**: We integrated an SMT-LIB2 Interface into Agam's compiler. 
* **Refinement Types**: You can type a variable as `{v: i32 | v != 0}`.
* When compiling, Agam translates your code into mathematical equations and runs the SMT Solver on them. It **proves** you will never divide by zero or access a bad array index, refusing to compile if an unsafe state is possible.

---

## 🧬 Phase 14 & 15: Direct LLVM Backend and Typed Scalar Lowering
### What changed?
Agam no longer has to rely only on the C transpilation path. We now have a direct textual LLVM IR backend for the current scalar/string subset, plus the first stage of width-aware scalar lowering so the backend can preserve explicit source types like `i64` instead of silently widening everything into one generic integer shape.

### What We Built:
* **Direct LLVM IR backend**: `agamc build --backend llvm` now emits `.ll` directly from MIR. When `clang` is available, that IR can be turned into a native executable without going through generated C.
* **First Cranelift JIT runtime**: `agamc run --backend jit` now lowers the current scalar MIR subset straight into machine code in memory, which gives Agam an immediate execution path even when you are not emitting a native file on disk.
* **Runtime-backed CLI helpers**: The LLVM and C backends both expose `argc()`, `argv()`, and `parse_int()` so Agam benchmarks can depend on real runtime inputs instead of compile-time constants.
* **Configurable LLVM target metadata**: The LLVM emitter supports `AGAM_LLVM_TARGET_TRIPLE` and `AGAM_LLVM_DATA_LAYOUT` so emitted IR can carry explicit module target information when desired.
* **Conservative ABI and pointer attributes**: The textual LLVM IR now emits a safe subset of attributes such as `noundef`, `nocapture`, `readonly`, and `local_unnamed_addr` where they are semantically justified.
* **Width-aware scalar lowering started**: Primitive scalar knowledge is now centralized in `agam_sema::types`, explicit scalar annotations survive HIR/MIR lowering, and the LLVM backend preserves signedness/width for built-in integer and float types instead of assuming one catch-all `i64`.
* **Broader induction proof and selective arithmetic flags**: The LLVM backend now tracks directly proven non-negative signed values, broadens strict seeded `+1` induction reasoning across hot while-loops, and emits `nuw` / `nsw` only on loop-counter increments whose guards make wraparound impossible under Agam's semantics.

### Why this matters
Agam's goal is premium systems performance **without** giving up the language's own semantics. That means we do **not** chase speed by pretending Agam has C++-style undefined signed overflow. The backend is being upgraded to earn those optimizations honestly: by preserving type width, then proving ranges, then emitting stronger LLVM facts only when they are actually true.

---

## 🧰 Phase 15: Developer Tooling, First Slice
### What changed?
The current compiler phases are now more usable for day-to-day development, not just backend experiments.

### What We Built:
* **Real `agamc fmt` command**: The formatter is no longer a stub. It now normalizes line endings, trailing whitespace, leading tab indentation, blank-line runs, and the final newline while preserving comments and existing source layout.
* **`fmt --check` workflow**: You can now gate formatting in CI or local pre-commit flows with `agamc fmt --check`.
* **Auto backend resolution**: `agamc build` and `agamc run` now support an `auto` backend mode and use it by default, preferring LLVM when `clang` is available and otherwise falling back to the C path.
* **Fast current-path builds**: `agamc build --fast` and `agamc run --fast` now resolve to the best currently-available native backend and force `-O3`, which makes the existing phases much better suited for premium development loops.
* **Fixed `run` path handling**: `agamc run` now executes binaries using the source file path instead of assuming the source lives in the current directory.

### What stays in future phases
The deeper compiler work still belongs in future development:
* Richer square-bound and derived range proof for LLVM.
* Smarter PGO / ThinLTO defaults once the next proof layer is trustworthy.
* Comment-aware structural formatting, full LSP features, and an Agam-native test runner.

### Essential next development phases
These are the highest-value performance and development features to build next:

1. **Phase 16: Portable Agam package + tiered runtime**
   * Define a platform-independent Agam package format.
   * Load that package through the runtime and JIT hot code on the target machine.
   * Keep native AOT backends as optional per-platform release targets.
2. **Phase 17: Persistent native code cache**
   * Add an on-disk cache keyed by Agam package hash, backend version, OS, CPU features, and runtime ABI.
   * Reuse previously compiled hot code to remove repeated startup compilation costs.
3. **Phase 18: Whole-program purity and effect metadata**
   * Promote call-cache decisions from manual hints to verified compiler facts.
   * Use the same effect metadata to unlock safer inlining, CSE, LICM, and auto-memoization.
4. **Phase 19: Value profiling + adaptive specialization**
   * Record hot argument shapes and constant-like values at runtime.
   * Clone and specialize hot functions only when the measured payoff is real.
5. **Phase 20: Escape analysis + stack promotion**
   * Move short-lived allocations out of the heap.
   * Feed stronger alias and lifetime facts into LLVM and the JIT.
6. **Phase 21: Incremental daemon + parallel compilation**
   * Keep parsed, typed, and lowered state warm across edits.
   * Parallelize frontend and backend work to make premium development loops feel immediate.

---

## 🔮 Future Scopes (Phase 11 and Beyond)

We are constantly expanding the boundaries of compiler tech:
1. **Capability-Based Security**: Instead of trusting OS permissions, functions will require exact unforgeable tokens (like `FileRead`), eliminating supply chain vulnerabilities.
2. **Content-Addressable Code**: We will replace fragile package managers (like NPM) by hashing ASTs using BLAKE3. Code is dependency-resolved by its mathematical hash across a decentralized network.
3. **Quantum Computing**: `Qubit` base types and Gates (H, X, Z) so you can write quantum operations adjacent to classical neural networks.
4. **Zero-Knowledge Proofs (ZKP)**: `#[zkp]` macros that compile mathematical logic straight into cryptographic circuits (zk-SNARKs) to prove data authenticity on decentralized networks.
5. **Premium Object System**: Agam already parses `struct`, `class`, `trait`, `impl`, field access, method calls, and struct literals. A dedicated future phase will finish that into a fully checked object model with constructors, `self`, visibility, composition/inheritance strategy, and premium dispatch semantics.
6. **Premium Ergonomics Layer**: A dedicated follow-up phase will make everyday Agam code feel lower-ceremony and more intentional by adding defaults where they improve flow, named/default arguments, cleaner constructor/property forms, and stronger syntax cohesion across scripts, libraries, and object-style APIs.

---

## ⚡ Real-World Benchmarks: Agam vs The World
As of **March 26, 2026**, Agam uses a **runtime-parameterized** benchmark suite so the optimizer cannot simply fold the whole workload away. The current fair comparison is run in **WSL** with the same numeric inputs for every implementation:

```bash
100000000 40 100000 100 10000000
```

The Agam benchmark was also upgraded to use explicit `i64` on the large accumulation paths so the comparison reflects the intended semantics instead of relying on accidental backend widening.

### Current fair timings (7-run trimmed average, WSL)

| Runtime | Total Time | Relative to Agam LLVM | Notes |
|---|---:|---:|---|
| **Agam LLVM -O3** | **0.4157 s** | **1.00x** | Direct LLVM IR emitted by `agam_codegen`, compiled with `clang -O3` in WSL |
| **C++ clang++ -O3** | 0.3946 s | 1.05x faster than Agam | Highest native baseline in current runs |
| **C++ g++ -O3** | 0.3912 s | 1.06x faster than Agam | GCC native baseline |

### What the benchmark means
* Agam is now in the same performance band as optimized C++ on this fair workload, and the current fair gap is still roughly **5%** against `clang++ -O3`.
* The latest LLVM work now covers both conservative sign-flow and broader strict-bound induction counters, which means hot loops like `sum_loop`, `fibonacci`, `matrix_multiply`, and `integrate_x2` can carry proven `add nuw nsw` increments while `count_primes` still keeps its safe `urem` lowering.
* The next major wins now come from **richer derived range proofs**: square-bound guards, stronger loop-carried facts, and then smarter use of the new PGO / ThinLTO knobs once those proofs are trustworthy.
* This is deliberate. Agam is aiming for native speed while preserving its own language model, not by inheriting C++ undefined behavior as a hidden optimization contract.

### Practical commands
```bash
# C backend
agamc build my_script.agam -O 3

# LLVM backend
agamc build my_script.agam --backend llvm -O 3

# LLVM backend with ThinLTO
agamc build my_script.agam --backend llvm -O 3 --lto thin

# LLVM backend with PGO instrumentation or profile use
agamc build my_script.agam --backend llvm -O 3 --pgo-generate profiles
agamc build my_script.agam --backend llvm -O 3 --pgo-use default.profdata

# JIT backend for in-memory execution
agamc run my_script.agam --backend jit

# Scalar call-result cache for repeated pure scalar calls
agamc run my_script.agam --backend jit --call-cache
agamc build my_script.agam --backend llvm -O 3 --call-cache

# Source-level call cache controls (default off)
# File-wide basic caching: place this in the file preamble before declarations.
@lang.feat.call_cache

# Function-local basic caching: cache only this function, or opt a function back out.
@lang.feat.call_cache
fn hot(x: i64) -> i64 { ... }

@lang.feat.no_call_cache
fn not_worth_caching(x: i64) -> i64 { ... }

# Experimental optimize mode: adaptive admission + bounded hot-entry replacement.
# The compiler warns when this annotation is used.
@experimental.call_cache.optimize

@experimental.call_cache.optimize
fn very_hot(x: i64) -> i64 { ... }

# Storage model:
# - JIT basic mode keeps direct cached entries only.
# - JIT optimize mode uses bounded hot-entry replacement plus a fixed-size pending-candidate buffer.
# - LLVM optimize mode uses bounded arrays plus a single repeated-input admission candidate.
# - None of these modes grow without an explicit cache capacity limit.

# Optional target metadata for LLVM emission
AGAM_LLVM_TARGET_TRIPLE=x86_64-pc-linux-gnu \
AGAM_LLVM_DATA_LAYOUT='e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128' \
agamc build my_script.agam --backend llvm -O 3

# Premium current-path development flow (`--fast` = `-O3` + host-native CPU tuning)
agamc fmt --check .
agamc build my_script.agam --fast
agamc run my_script.agam --fast
```

---
*Agam is a journey to teach, build, and push the performance of what a modern systems programming language can achieve for Data Science and Machine Learning.*
