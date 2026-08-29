# Front Matter: Preface & Pedagogical Roadmap

## Title Page
**Engineering the Agam Compiler**  
*From Systems Foundations to Advanced LLVM Infrastructure*

---

## 1. Preface

Compilers are often viewed as mystifying software systems reserved for specialist theoretical computer scientists. However, modern industrial compilers are disciplined engineering pipelines built upon structured transformations, graph algorithms, and formal execution contracts.

The purpose of this textbook is to provide a complete, accessible, yet rigorous guide to compiler engineering. By pairing classic foundational literature with the concrete implementation of the **Agam Compiler** (`crates/{core,middle,backends,runtime,tooling}`), readers learn not only *why* compiler algorithms work theoretically, but *how* they are implemented in production Rust code.

---

## 2. Theoretical Framework & Classic Literature

This book integrates concepts across seven landmark compiler engineering works:

1. **The C Programming Language (K&R)**: Teaches the low-level machine execution model, pointer arithmetic, memory alignment, and standard C ABI calling conventions.
2. **Crafting Interpreters (Robert Nystrom)**: Demonstrates modern, readable frontend implementation including lexing, Pratt parsing, and object mechanics.
3. **Language Implementation Patterns (Terence Parr)**: Provides structural design patterns for AST trees, symbol tables, nested lexical scopes, and type checking.
4. **Engineering a Compiler (Keith D. Cooper & Linda Torczon)**: Explores modern intermediate representations (IR), Control Flow Graphs (CFG), SSA form, register allocation, and instruction scheduling.
5. **Modern Compiler Implementation in C (Andrew W. Appel)**: Establishes pipelines for translating high-level functional concepts into imperative IRs and target assembly.
6. **LLVM Code Generation: A Deep Dive (Quentin Colombet)**: Details LLVM code generation infrastructure, MachineIR (MIR), SelectionDAG/GlobalISel, TableGen files, and backend target generation.
7. **LLVM Techniques, Tips, and Best Practices (Kai Nacke & Amy Kwan)**: Demonstrates practical C++ LLVM API usage, AST-to-LLVM-IR translation, PassManager configuration, and JIT compilation.

---

## 3. Pedagogical Roadmaps

Depending on your prior experience, follow these recommended reading tracks:

### Track 1: Beginner (Systems & Frontend Foundations)
- **Part I**: Chapters 1–2 (C memory model, calling conventions, stack frames)
- **Part II**: Chapters 3–6 (Lexing, Pratt parsing, AST, symbol resolution, type checking)

### Track 2: Intermediate (Compiler Middle-End & Optimization)
- **Part II**: Chapters 5–6 (AST nodes & semantic checking)
- **Part III**: Chapters 7–10 (HIR/MIR, Control Flow Graphs, SSA form, middle-end optimizations)

### Track 3: Advanced (LLVM Backend Engineering & Compiler Architecture)
- **Part III**: Chapters 8–10 (SSA transformations & functional lowerings)
- **Part IV**: Chapters 11–14 (LLVM IR emission, PassManager, GlobalISel, register allocation)
- **Part V**: Chapters 15–18 (Agam compiler architecture, daemon compilation, sandboxing, Indic design principles)
