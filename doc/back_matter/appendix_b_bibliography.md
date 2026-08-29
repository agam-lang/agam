# Appendix B: Annotated Bibliography & Reading List

> **Target Audience**: Compiler Architects, Systems Engineers & Programming Language Researchers

---

## 1. Classical Compiler Engineering & IR Theory

1. **Cooper, Keith D., and Linda Torczon.** *Engineering a Compiler*. 3rd ed., Morgan Kaufmann, 2022.
   - *Core Grounding*: Definitive reference for High-Level and Medium-Level IRs, Dominance Frontiers, Static Single Assignment (SSA) form, GVN, SCCP, and loop optimization passes.
2. **Appel, Andrew W.** *Modern Compiler Implementation in C / ML*. Cambridge University Press, 1998.
   - *Core Grounding*: Functional semantics lowering, def-use chains, liveness analysis, iterated register coalescing, and CPS/closure conversion mechanics.
3. **Muchnick, Steven S.** *Advanced Compiler Design and Implementation*. Morgan Kaufmann, 1997.
   - *Core Grounding*: Classical dataflow analysis frameworks, interprocedural optimizations, strength reduction, and code motion safety criteria.
4. **Aho, Alfred V., Monica S. Lam, Ravi Sethi, and Jeffrey D. Ullman.** *Compilers: Principles, Techniques, and Tools* (The Dragon Book). 2nd ed., Addison-Wesley, 2006.
   - *Core Grounding*: Lexical analysis, LR parsing, syntax-directed translation, symbol tables, and code generation basics.
5. **Nystrom, Robert.** *Crafting Interpreters*. Genever Benning, 2021.
   - *Core Grounding*: Architectural blueprint for clean Pratt parsing, bytecode VMs, hash table dynamics, and developer-friendly compiler ergonomics.
6. **Parr, Terence.** *Language Implementation Patterns: Create Your Own Domain-Specific and General-Purpose Languages*. Pragmatic Bookshelf, 2009.
   - *Core Grounding*: Pattern catalog for AST structures, symbol resolution, nested scope graphs, and polymorphic visitor patterns.

---

## 2. LLVM & Modern Code Generation Backends

7. **Colombet, Quentin.** *LLVM Code Generation: A Deep Dive into Compiler Backend Development*. Packt Publishing, 2024.
   - *Core Grounding*: GlobalISel pipeline architecture, SelectionDAG lowering, MachineIR (MIR), TableGen (`.td`), and target register/instruction definitions.
8. **Nacke, Kai, and Amy Kwan.** *LLVM Techniques, Tips, and Best Practices*. Packt Publishing, 2021.
   - *Core Grounding*: Modern LLVM PassManager, LLVM C/C++ API builder patterns, ORC JIT v2 engines, and Cross-Target compilation setup.
9. **Lattner, Chris, and Vikram Adve.** *LLVM: A Compilation Framework for Lifelong Program Analysis & Transformation*. CGO, 2004.
   - *Core Grounding*: Original architectural foundation of LLVM's universal SSA IR and modular optimization strategy.
10. **Lattner, Chris, et al.** *MLIR: Scaling Compiler Infrastructure for Domain Specific Computations*. IEEE/ACM CGO, 2021.
    - *Core Grounding*: Multi-level intermediate representation dialect design, progressive lowering, and polyhedral tile transformations.

---

## 3. GPU Computing, Hardware Acceleration & Parallel Systems

11. **Kirk, David B., and Wen-mei W. Hwu.** *Programming Massively Parallel Processors: A Hands-on Approach*. 4th ed., Morgan Kaufmann, 2022.
    - *Core Grounding*: SIMT execution models, warp divergent branch mitigation, memory coalescing, shared memory tiling, and tensor core utilization.
12. **Khronos Group.** *SPIR-V Specification (Provisional & 1.5/1.6 Core)*. Khronos Open Standard, 2023.
    - *Core Grounding*: Binary intermediate language format, logical memory models, capability negotiation, and cooperative matrix extensions (`SPV_KHR_cooperative_matrix`).
13. **NVIDIA Corporation.** *NVIDIA Hopper Architecture In-Depth & PTX ISA Reference Manual*. NVIDIA Developer Documentation, 2023.
    - *Core Grounding*: Tensor Memory Accelerator (TMA) hardware copy descriptors, asynchronous multi-stage pipelines, and warp group matrix multiply-accumulate (WGMMA).
14. **Herlihy, Maurice, and Nir Shavit.** *The Art of Multiprocessor Programming*. 2nd ed., Morgan Kaufmann, 2020.
    - *Core Grounding*: Lock-free synchronization, work-stealing deques, memory consistency models, and wait-free concurrent ring buffers.

---

## 4. Programming Language Semantics, Types & Security

15. **Pierce, Benjamin C.** *Types and Programming Languages* (TAPL). MIT Press, 2002.
    - *Core Grounding*: Formal type systems, bidirectional type checking, subtyping, and operational semantics.
16. **Plotkin, Gordon D., and John Power.** *Algebraic Operations and Generic Effects*. Applied Categorical Structures, 2003.
    - *Core Grounding*: Theoretical foundation of algebraic effects and handlers as modular control abstractions.
17. **Bauer, Andrej, and Matija Pretnar.** *Programming with Algebraic Effects and Handlers*. Journal of Logical and Algebraic Methods in Programming, 2015.
    - *Core Grounding*: Practical compilation strategies for stackless effect state machines and multishot/single-shot continuations.
18. **Kernighan, Brian W., and Dennis M. Ritchie.** *The C Programming Language*. 2nd ed., Prentice Hall, 1988.
    - *Core Grounding*: Stack frame structures, memory alignment, pointers, C ABI interoperability, and low-level machine execution.
19. **Anderson, Ross.** *Security Engineering: A Guide to Building Dependable Distributed Systems*. 3rd ed., Wiley, 2020.
    - *Core Grounding*: Capability security models, constant-time cryptography, secret zeroization, and OS-level sandboxing.

---

## 5. Classical Indic Grammar & Formal Linguistics

20. **Pāṇini.** *Aṣṭādhyāyī* (ca. 4th Century BCE). Edited and translated by S. M. Katre, Motilal Banarsidass, 1989.
    - *Core Grounding*: The foundational generative formal grammar of Sanskrit, featuring ~4,000 algorithmic algebraic rules, context-sensitive rule application, and metalinguistic markers (It-saṁjñā).
21. **Tolkāppiyar.** *Tolkāppiyam* (ca. 3rd Century BCE – 2nd Century CE). Translated by S. Ilakkuvanar, Kural Neri Publishing, 1966.
    - *Core Grounding*: Ancient Tamil grammatical treatise outlining structural phonology (Eluttu), syntax/semantics (Col), and thematic discourse principles (Porul).
22. **Kiparsky, Paul.** *Some Consequences of Pāṇini's Rule of Rule-Ordering*. Journal of Indian Philosophy, 1982.
    - *Core Grounding*: Analysis of formal rule ordering, specificity override (Niravakāśa / Apavāda), and precedence resolution in formal rewriting systems.
