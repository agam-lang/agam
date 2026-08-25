# Appendix C: Comprehensive Glossary of Technical Terms

> **Domain Coverage**: Compiler Engineering, Middle-End Optimization, Indic Grammatical Architecture, GPU/Acceleration, Concurrency, and Security

---

## 1. Core Compiler & Frontend Engineering

- **Abstract Syntax Tree (AST)**: A hierarchical tree representing the syntactic structure of source code, abstracting away concrete delimiters, punctuation, and whitespace while preserving operator precedence and nested declarations.
- **Application Binary Interface (ABI)**: The low-level machine contract defining parameter register allocation, stack alignment, struct layout padding, and name mangling between independently compiled modules.
- **Backus-Naur Form (BNF / EBNF)**: Formal metasyntax notations used to express context-free grammars defining valid language sentence structures.
- **Bidirectional Type Inference**: A type checking methodology that alternates between *synthesizing* types from expressions and *checking* expressions against expected types, reducing mandatory type annotations.
- **Closure Conversion**: The middle-end transformation that rewrites first-class functions capturing lexical variables into explicit environment structs paired with static function pointers.
- **Concrete Syntax Tree (CST)**: A lossless parse tree retaining all source tokens, comments, and whitespace, used by formatters and LSP servers for faithful source roundtripping.
- **Def-Use Chain**: Data structures connecting an SSA variable's definition statement to all instructions that consume its value.
- **Lexical Analysis (Scanning)**: The first compiler phase transforming raw UTF-8 byte streams into a linear sequence of typed tokens with source span locations.
- **Monomorphization**: The compile-time expansion of generic types and parameterized functions into distinct concrete type instantiations, eliminating runtime dispatch overhead.
- **Nyāya 4-Part Diagnostic Model**: Agam's error reporting philosophy structured around Thesis (*Pratijñā*), Reason (*Hetu*), Example (*Udāharaṇa*), and Application (*Upanaya*).
- **Pratt Parsing (Top-Down Operator Precedence)**: An elegant parsing technique assigning left and right binding powers to tokens to resolve infix, prefix, and postfix expressions in $O(N)$ time without deep recursion.
- **Semantic Analysis (Sema)**: The phase validating symbol scoping, name resolution, type consistency, mutability guarantees, and effect propagation.
- **Source Span**: A compact structure `(SourceId, StartOffset, EndOffset)` pinning every AST node to exact line and column coordinates in source text.
- **Symbol Table**: A scoped hierarchical dictionary mapping textual identifier names to their types, storage locations, and visibility attributes.
- **Type Sandhi**: Agam's type unification and coercion engine, inspired by Sanskrit phonological sandhi rules, resolving union types, promotions, and subtyping relationships.

---

## 2. Middle-End & SSA Optimization

- **Basic Block (BB)**: A straight-line sequence of instructions with a single entry point (the first instruction) and a single exit point (the terminating jump, branch, or return).
- **Control Flow Graph (CFG)**: A directed graph $G = (V, E)$ where vertices $V$ represent basic blocks and edges $E$ represent possible control flow transitions.
- **Dead Code Elimination (DCE)**: An optimization pass eliminating instructions whose computed results have no reachable side effects or consumers.
- **Dominator Tree**: A tree where node $A$ is the immediate dominator of node $B$ ($A = idom(B)$) if every execution path from entry to $B$ must pass through $A$.
- **Dominance Frontier ($DF$)**: For a node $X$, the set of all nodes $Y$ such that $X$ dominates a predecessor of $Y$, but does not strictly dominate $Y$ itself; used for optimal $\phi$-node placement in SSA conversion.
- **Function Inlining**: The optimization replacing a function call site with the body of the called function, eliminating calling overhead and exposing intra-procedural optimization opportunities.
- **Global Value Numbering (GVN)**: An SSA-based optimization assigning canonical value identifiers to redundant expressions across distinct basic blocks to eliminate common subexpressions.
- **High-Level Intermediate Representation (HIR)**: An AST-adjacent desugared IR where syntactic sugar (pattern matching, loops, `?` operators) is normalized into primitive control nodes.
- **Loop Invariant Code Motion (LICM)**: An optimization pass identifying expressions within a loop whose operands never change across iterations and hoisting them into the loop pre-header block.
- **Loop Unrolling**: Replicating a loop body $N$ times to amortize branch prediction penalties, reduce loop counter increments, and widen SIMD/instruction-level scheduling windows.
- **Medium-Level Intermediate Representation (MIR)**: A control-flow-centric SSA intermediate representation consisting of basic blocks, explicit terminators, and target-agnostic instructions.
- **Phi Node ($\phi$-node)**: A synthetic SSA instruction placed at CFG join points that selects a variable's value based on which predecessor block control flowed from.
- **Sparse Conditional Constant Propagation (SCCP)**: A lattice-based optimization that simultaneously discovers unreachable basic blocks and propagates compile-time constant values across the CFG.
- **Static Single Assignment (SSA)**: A property of intermediate representations guaranteeing that every variable is assigned a value exactly once, simplifying dataflow analysis.
- **Strength Reduction**: An optimization replacing computationally expensive operations with cheaper equivalents (e.g., replacing loop induction multiplication with repeated additions or shifts).
- **Tail Call Optimization (TCO)**: Reusing the caller's stack frame when the final operation is a function call, enabling unbounded recursion in $O(1)$ stack space.

---

## 3. Backend, Code Generation & JIT

- **C11 Portable Emitter**: A fallback backend that translates Agam MIR into ANSI C11 source code, providing universal portability across platforms lacking LLVM support.
- **Cranelift**: A fast, lightweight native code generator designed for WebAssembly runtimes and interactive JIT compilation engines.
- **Fat-Binary (.agpkg)**: A unified distribution package containing multi-architecture binary slices (x86_64, ARM64, WASM) with dynamic host runtime dispatch.
- **GlobalISel**: LLVM's modern global instruction selection framework replacing legacy SelectionDAG with a multi-pass pipeline over MachineIR (IRTranslator → Legalizer → RegBankSelect → InstructionSelect).
- **Iterated Register Coalescing (IRC)**: Appel-George graph-coloring algorithm that minimizes register spills and eliminates register-to-register copy instructions.
- **Just-In-Time (JIT) Compilation**: Compiling intermediate code into native host machine instructions in memory during runtime execution.
- **LLVM Bitcode (.bc)**: A binary, bitstream representation of LLVM Intermediate Representation optimized for fast compiler ingestion and Link-Time Optimization (LTO).
- **MachineIR (MIR - LLVM)**: LLVM's target-dependent representation of instructions and virtual/physical registers before emitting assembly or machine code.
- **Target Triplet**: A standard string `<arch>-<vendor>-<os>-<env>` (e.g., `x86_64-pc-windows-msvc`) defining the target compilation environment.
- **Target Pack**: A modular SDK distribution containing sysroot headers, pre-compiled runtime static libraries, and linker scripts for cross-compilation.

---

## 4. GPU, TMA & Hardware Acceleration

- **AsyncPipelineStage**: A runtime/codegen synchronization token managing multi-stage asynchronous data transfers between global VRAM and shared memory.
- **Cooperative Matrix**: A hardware-accelerated matrix multiplication-accumulation primitive (`SPV_KHR_cooperative_matrix`) executing on GPU Tensor Cores, Intel XMX, or AMD Matrix Cores.
- **Extent**: A multi-dimensional coordinate vector describing the bounding dimensions of a sub-tensor slice.
- **PartitionView**: A zero-copy, strided view into multi-dimensional tensor storage enabling flexible sub-volume slicing without memory copies.
- **Single Instruction, Multiple Threads (SIMT)**: GPU execution architecture where instructions are issued simultaneously across multiple SIMD thread lanes (Warps/Wavefronts).
- **SPIR-V**: Standard Portable Intermediate Representation; Khronos Group's cross-vendor binary intermediate language for graphics and parallel compute.
- **Tensor Memory Accelerator (TMA)**: NVIDIA Hopper+ dedicated hardware copy engine executing multi-dimensional tensor memory transfers directly between global memory and shared memory without SM compute overhead.
- **Tile Abstraction (`Tile<T, M, N>`)**: A compile-time-sized 2D matrix fragment held collaboratively in shared memory or register files for high-throughput tiled GEMM and convolution algorithms.
- **Warp / Wavefront**: The fundamental hardware scheduling unit of GPU execution (typically 32 threads in NVIDIA/Intel, 32/64 threads in AMD).

---

## 5. Concurrency, Systems & Security

- **Algebraic Effect Handler**: A modular control flow abstraction separating the invocation of side effects (`perform`) from their concrete runtime implementation (`handle`), supporting resumptions (`resume`).
- **ChaCha20-Poly1305**: An authenticated stream cipher and AEAD construction providing high-performance cryptographic confidentiality and integrity.
- **Chāṇakya Durdharṣa Sandbox**: Agam's OS-level process isolation layer utilizing Windows JobObjects and Linux `prctl`/cgroups to limit memory, CPU, and file system capabilities.
- **Constant-Time Cryptography**: Code sequences engineered to execute in identical CPU clock cycles regardless of secret input values, preventing timing side-channel attacks.
- **FastRingBuffer**: A lock-free, cache-line-aligned single-producer single-consumer (SPSC) circular queue for ultra-low latency inter-thread communication.
- **Nursery**: A scoped structured concurrency block that guarantees all spawned child tasks complete or cancel before control leaves the lexical scope.
- **Secret Zeroization**: Overwriting memory containing sensitive keys or tokens with zeros immediately upon variable drop using volatile compiler-barrier memory writes.
- **Taint Tracking**: Static compiler analysis tracking untrusted user input data across function boundaries to prevent injection vulnerabilities.
- **Work-Stealing Scheduler**: A multi-threaded task scheduling algorithm where idle worker threads steal runnable tasks from the deques of busy threads.

---

## 6. Indic Grammatical & Linguistic Concepts

- **Apavāda (Special Rule Override)**: The grammatical meta-rule dictating that a more specific rule takes precedence over a general rule (*utsarga*), foundational to Agam's pattern matching and macro expansion priorities.
- **Aṣṭādhyāyī**: Pāṇini's foundational Sanskrit grammar comprising ~4,000 algorithmic rules, demonstrating formal rewrite systems 2,400 years before Chomsky.
- **Dhātu (Verbal Root)**: Atomic semantic root verbs categorized into ten classes (*gaṇas*), utilized in Agam's standard library naming taxonomy for mathematical and computational operations.
- **Kāraka (Semantic Roles)**: Pāṇini's formal framework for semantic relations between actions and participants (Agent, Object, Instrument, Destination, Source, Locus), mapped to function parameter signatures.
- **Pratyāhāra**: Concise shorthand notation condensing ranges of phonemes or types using bounding markers (e.g., *aṇ*, *hal*), inspiring Agam's type constraint syntax.
- **Tolkāppiyam**: The earliest classical Tamil grammatical treatise, formulating orthography (*Eluttu*), morphological syntax (*Col*), and expressive semantics (*Porul*).
- **Vibhakti**: Grammatical case inflections indicating syntactical relations and semantic roles within expressions.
