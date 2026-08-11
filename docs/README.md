# Engineering the Agam Compiler & Language Programming Guide

*A Comprehensive Textbook, Architecture Reference & Language User Guide*

---

## 📖 Book Overview

This textbook provides a comprehensive, structured guide to building modern optimizing compilers and programming in **Agam**. Using the production **Agam Compiler** (`crates/{core,middle,backends,runtime,tooling}`) as its primary implementation model, this volume bridges foundational literature with industrial software engineering practice.

### Interactive Online & Offline HTML Book
Build or serve this entire documentation suite as an interactive, searchable web book using **mdBook**:
```bash
cd doc
mdbook serve --open
```

### Core Literature Integration

Every chapter integrates theoretical principles and practical patterns from seven landmark texts in systems programming and compiler design:

- **The C Programming Language (K&R)** by Brian W. Kernighan & Dennis M. Ritchie
- **Crafting Interpreters** by Robert Nystrom
- **Language Implementation Patterns** by Terence Parr
- **Engineering a Compiler** by Keith D. Cooper & Linda Torczon
- **Modern Compiler Implementation in C (Tiger Book)** by Andrew W. Appel
- **LLVM Code Generation: A Deep Dive** by Quentin Colombet
- **LLVM Techniques, Tips, and Best Practices** by Kai Nacke & Amy Kwan

---

## 📚 Complete Table of Contents

### Front Matter
- **[Preface & Reader Roadmap](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/front_matter/00_preface_and_roadmap.md)**: Book objectives, literature mapping, and pedagogical tracks.
- **[One-Page Syntax Cheat Sheet](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/CHEATSHEET.md)**: Quick reference card for Agam syntax, tensors, effects, and CLI tools.

---

### Part I: Systems Programming & Low-Level Foundations
*Grounded in Kernighan & Ritchie (K&R C)*

- **[Chapter 1: The C Execution & Memory Model](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_1_foundations/ch01_c_memory_model.md)**: Stack vs. Heap layout, pointer arithmetic, struct alignment, and padding calculations.
- **[Chapter 2: Hardware Architecture, Calling Conventions & System ABIs](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_1_foundations/ch02_calling_conventions.md)**: System V AMD64 vs Windows x64 ABIs, register usage, stack frame construction, and runtime C ABI bindings.

---

### Part II: Language Design & Frontend Mechanics
*Grounded in Nystrom (Crafting Interpreters) & Parr (Language Implementation Patterns)*

- **[Chapter 3: Lexical Analysis & Token Scanning](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_2_frontend/ch03_lexical_analysis.md)**: Token streams, UTF-8 scanning, and source position tracking (`Span`, `SourceId`).
- **[Chapter 4: Parsing Theory & Pratt Parsing Mechanics](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_2_frontend/ch04_pratt_parsing.md)**: Top-down operator precedence parsing, binding powers, and statement parsing.
- **[Chapter 5: Abstract Syntax Trees & Grammar Representation](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_2_frontend/ch05_ast_design.md)**: Recursive AST design, expression variants, pattern match syntax, and effect nodes.
- **[Chapter 6: Symbol Tables, Lexical Scopes & Type Inference](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_2_frontend/ch06_sema_and_types.md)**: Nested scope graphs, symbol resolution, type propagation, and effect checking.

---

### Part III: Compiler Architecture & Optimization Theory
*Grounded in Cooper & Torczon (Engineering a Compiler) & Appel (Tiger Book)*

- **[Chapter 7: High-Level & Medium-Level Intermediate Representations (HIR & MIR)](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_3_middle_end/ch07_hir_and_mir.md)**: AST lowering, desugaring passes, HIR layout, and MIR control flow structure.
- **[Chapter 8: Control Flow Graphs & Static Single Assignment (SSA) Form](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_3_middle_end/ch08_cfg_and_ssa.md)**: Basic Blocks, $\phi$-node placement, dominance frontiers, and definition-use chains.
- **[Chapter 9: Middle-End Optimization Passes](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_3_middle_end/ch09_optimization_passes.md)**: Constant folding, dead code elimination (DCE), loop invariant code motion (LICM), and function inlining.
- **[Chapter 10: Lowering Functional & Effectful Semantics](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_3_middle_end/ch10_functional_lowering.md)**: Closure conversion, decision-tree pattern matching, and algebraic effect suspension frames.

---

### Part IV: LLVM Backend & Code Generation Infrastructure
*Grounded in Colombet (LLVM Code Generation) & Nacke & Kwan (LLVM Techniques)*

- **[Chapter 11: Emitting Textual & Bitcode LLVM IR](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_4_llvm_backend/ch11_llvm_ir_codegen.md)**: Mapping MIR to LLVM IR, context setup, builder patterns, and bitcode emission.
- **[Chapter 12: Modern PassManager & In-Process JIT Engines](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_4_llvm_backend/ch12_passmanager_and_jit.md)**: Optimization pass pipelines (-O0 to -O3), ORC JIT, and Cranelift execution.
- **[Chapter 13: LLVM Backend Architecture: SelectionDAG, GlobalISel & MachineIR](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_4_llvm_backend/ch13_llvm_backend_deep_dive.md)**: SelectionDAG vs. GlobalISel pipelines, MachineIR (MIR layer), and TableGen (`.td`) files.
- **[Chapter 14: Register Allocation Algorithms & Machine Code (MC) Layer](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_4_llvm_backend/ch14_register_allocation.md)**: Graph coloring vs greedy allocation, spilling, MC layer, and native binary generation.

---

### Part V: The Agam Compiler Architecture & Features
*Production System Architecture & Advanced Design*

- **[Chapter 15: End-to-End Agam Compiler Pipeline Walkthrough](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_5_agam_architecture/ch15_compiler_pipeline.md)**: Full source-to-binary compilation lifecycle and driver coordination (`agam_driver`).
- **[Chapter 16: Advanced Language Features: Native Tensors & Algebraic Effects](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_5_agam_architecture/ch16_language_features.md)**: Hardware-accelerated tensor primitives and algebraic effect handler implementation.
- **[Chapter 17: Incremental Compilation Daemon & Sandboxed Runtime](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_5_agam_architecture/ch17_daemon_and_sandbox.md)**: `DaemonSession` warm state caching, snapshot invalidation, and OS-level sandboxing (JobObject/prctl).
- **[Chapter 18: Indic Grammatical Design Principles (Pāṇini & Tolkāppiyam)](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_5_agam_architecture/ch18_indic_grammar_rules.md)**: Pāṇini's Aṣṭādhyāyī and Tolkāppiyam rules: Dhātu root verbs, Vibhakti roles, and Type Sandhi rules.

---

### Part VI: The Agam Language Programming Guide
*Complete Application Programming Guide (Basic to Advanced)*

- **[Chapter 19: Getting Started & Basics of Agam](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_6_language_guide/ch19_getting_started_and_basics.md)**: Hello World, variables, mutability, primitives, function signatures.
- **[Chapter 20: Control Flow, Structs & Collections](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_6_language_guide/ch20_control_flow_and_structures.md)**: Conditionals, loops, `struct`, methods, arrays, tuples.
- **[Chapter 21: Tagged Union Enums, Pattern Matching & Error Handling](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_6_language_guide/ch21_enums_patterns_and_errors.md)**: Payload enums, pattern matching (`match`), `Option[T]`, `Result[T, E]`.
- **[Chapter 22: First-Class Tensors & Numerical AI Operations](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_6_language_guide/ch22_tensors_and_numerical_ai.md)**: `Tensor` primitives, matrix multiplication, shape broadcasting, neural net layers.
- **[Chapter 23: Algebraic Effect Handlers in Depth](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_6_language_guide/ch23_algebraic_effects_in_depth.md)**: `effect`, `perform`, `handle`, `resume`, async non-blocking control flow.
- **[Chapter 24: Modules, Package Management (`agam.toml`) & FFI](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_6_language_guide/ch24_modules_packages_and_ffi.md)**: Package manifests, imports, C & Python FFI bindings.
- **[Chapter 25: Metaprogramming, REPL, Notebooks & Tooling](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_6_language_guide/ch25_metaprogramming_and_tooling.md)**: `agamc repl`, headless agent execution (`agamc exec`), `agamc fmt`, `agamc lint`.
- **[Chapter 25b: Real-World Agam Code Cookbook](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_6_language_guide/ch25b_cookbook.md)**: Production recipes for Web API with Effects, ML Tensor Pipelines, CLI tools.

---

### Part VII: Advanced Tooling, Testing & Ecosystem Engineering
*Production Infrastructure & Tooling Architecture*

- **[Chapter 26: Diagnostic Engineering, Spans & Error Recovery](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_7_ecosystem_and_tooling/ch26_diagnostics_and_spans.md)**: Diagnostic data models (`agam_errors`), span tracking, snippet rendering, error recovery.
- **[Chapter 27: Testing Methodologies, Fuzzing & Differential Verification](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_7_ecosystem_and_tooling/ch27_compiler_testing_and_fuzzing.md)**: Multi-tier testing, test harnesses (`agam_test`), JIT vs LLVM differential testing.
- **[Chapter 28: Language Server Protocol (LSP) Architecture](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_7_ecosystem_and_tooling/ch28_lsp_architecture.md)**: JSON-RPC server (`agam_lsp`), publishDiagnostics, hover tooltips, go-to-definition, autocomplete.
- **[Chapter 29: Source Code Formatting Engine Architecture (`agam_fmt`)](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_7_ecosystem_and_tooling/ch29_formatter_engine.md)**: Formatter pipeline (`agam_fmt`), CST traversal, indentation rules, line breaking.
- **[Chapter 30: Cross-Compilation, Target Triplets & Target Packs](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_7_ecosystem_and_tooling/ch30_cross_compilation_targets.md)**: Target triplets, Android ARM64 target packs, cross-linking staging (`agam_pkg`).
- **[Chapter 31: Compiler Profiling & Performance Measurement](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/part_7_ecosystem_and_tooling/ch31_profiling_and_benchmarking.md)**: Profiling harnesses (`agam_profile`), throughput measurement, flamegraph phase timings.

---

### Back Matter
- **[Appendix A: Comprehensive Agam Crate Map](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/back_matter/appendix_a_crate_map.md)**: Physical crate boundaries, dependencies, and API surfaces.
- **[Appendix B: Annotated Bibliography & Reading List](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/back_matter/appendix_b_bibliography.md)**: Detailed references and study guides.
- **[Appendix C: Glossary of Compiler & Indic Design Terms](file:///c:/Users/ksvik/Projects/Agam-Lang/doc/back_matter/appendix_c_glossary.md)**: Glossary of compiler engineering and Indic grammatical terminology.
