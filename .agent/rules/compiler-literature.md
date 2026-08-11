# Compiler Literature Architecture & Design Rule

> **Scope**: Mandatory architectural design rules for all compiler passes, frontend parser extensions, middle-end optimizations, and backend code generators in the Agam Compiler (`crates/{core,middle,backends,runtime,tooling}`).

---

## 1. Core Literature Directives

All AI agents (Claude, Gemini, Codex, Antigravity) and human contributors MUST adhere to the design patterns and theoretical models established in the seven core compiler literature texts:

### 1. Language Design & Frontend Mechanics
- **Crafting Interpreters by Robert Nystrom**:
  - *Applicability*: `agam_lexer`, `agam_parser`, `agam_ast`.
  - *Mandate*: Use top-down operator precedence (Pratt parsing) for expression evaluation. Maintain byte position tracking via `Span` and `SourceId` for error resilience.
- **Language Implementation Patterns by Terence Parr**:
  - *Applicability*: `agam_sema`.
  - *Mandate*: Use structured scope graphs for nested symbol tables. Separate AST traversals from tree definitions using visitor or folder traits. Perform bidirectional static type checking and effect verification.

### 2. Compiler Architecture & Optimization Theory
- **Engineering a Compiler by Keith D. Cooper & Linda Torczon**:
  - *Applicability*: `agam_hir`, `agam_mir`, `agam_mir::opt`.
  - *Mandate*: Structure MIR functions as Control Flow Graphs (CFG) of Basic Blocks with explicit terminators (`Goto`, `Branch`, `SwitchInt`, `Return`). Enforce Static Single Assignment (SSA) form with minimal $\phi$-nodes placed via dominance frontier analysis.
- **Modern Compiler Implementation in C by Andrew W. Appel**:
  - *Applicability*: `agam_hir` $\rightarrow$ `agam_mir` lowering.
  - *Mandate*: Transform anonymous functions into explicit closure environment structures. Desugar pattern matching into explicit decision trees. Convert algebraic effects (`perform`/`handle`) into stack-frame yield nodes.

### 3. Working with LLVM Backends & Infrastructure
- **LLVM Code Generation: A Deep Dive into Compiler Backend Development by Quentin Colombet**:
  - *Applicability*: `agam_codegen`.
  - *Mandate*: Follow MachineIR (MIR) and GlobalISel / SelectionDAG instruction selection pipelines. Define target hardware encodings using TableGen (`.td`) files. Implement graph coloring or greedy register allocation models.
- **LLVM Techniques, Tips, and Best Practices by Kai Nacke & Amy Kwan**:
  - *Applicability*: `agam_codegen`, `agam_jit`.
  - *Mandate*: Use LLVM Context, Module, and Builder patterns for textual/bitcode LLVM IR emission. Configure modern `PassManager` optimization pipelines (-O0 to -O3). Execute interactive code via ORC JIT / Cranelift engines.

### 4. Low-Level Systems Foundations
- **The C Programming Language (K&R) by Brian W. Kernighan & Dennis M. Ritchie**:
  - *Applicability*: `agam_runtime`, `agam_codegen` (C fallback).
  - *Mandate*: Maintain strict System V AMD64 and Windows x64 ABI compliance. Align struct fields explicitly to boundary multiples. Ensure safe memory management and C FFI bindings.

---

## 2. Verification Requirement
Any code contribution adding compiler passes or modifying language syntax MUST cite the corresponding literature design pattern in the PR/commit documentation.
