# Summary

[Book Overview](README.md)
[Language Requirements Specification](LANGUAGE_REQUIREMENTS_SPECIFICATION.md)
[Object System Architecture](OBJECT_SYSTEM_ARCHITECTURE.md)
[Parser & Grammar Specification](PARSER_GRAMMAR_SPECIFICATION.md)
[Async Coroutine Architecture](ASYNC_COROUTINE_ARCHITECTURE.md)
[Runtime Systems Specification](RUNTIME_SYSTEMS_SPECIFICATION.md)
[VM & JIT Compiler Specification](VM_JIT_COMPILER_SPECIFICATION.md)
[Multi-Target Codegen Specification](MULTI_TARGET_CODEGEN_SPECIFICATION.md)
[Preface & Reader Roadmap](front_matter/00_preface_and_roadmap.md)
[One-Page Syntax Cheat Sheet](CHEATSHEET.md)

# Part I: Systems Programming Foundations
- [Chapter 1: The C Execution & Memory Model](part_1_foundations/ch01_c_memory_model.md)
- [Chapter 2: Calling Conventions & System ABIs](part_1_foundations/ch02_calling_conventions.md)

# Part II: Language Design & Frontend Mechanics
- [Chapter 3: Lexical Analysis & Token Scanning](part_2_frontend/ch03_lexical_analysis.md)
- [Chapter 4: Parsing Theory & Pratt Mechanics](part_2_frontend/ch04_pratt_parsing.md)
- [Chapter 5: Abstract Syntax Trees & Grammar](part_2_frontend/ch05_ast_design.md)
- [Chapter 6: Symbol Tables & Type Inference](part_2_frontend/ch06_sema_and_types.md)

# Part III: Compiler Architecture & Optimization Theory
- [Chapter 7: High-Level & Medium-Level IRs](part_3_middle_end/ch07_hir_and_mir.md)
- [Chapter 8: Control Flow Graphs & SSA Form](part_3_middle_end/ch08_cfg_and_ssa.md)
- [Chapter 9: Middle-End Optimization Passes](part_3_middle_end/ch09_optimization_passes.md)
- [Chapter 10: Lowering Functional & Effect Semantics](part_3_middle_end/ch10_functional_lowering.md)

# Part IV: LLVM Backend & Infrastructure
- [Chapter 11: Emitting Textual & Bitcode LLVM IR](part_4_llvm_backend/ch11_llvm_ir_codegen.md)
- [Chapter 12: Modern PassManager & JIT Engines](part_4_llvm_backend/ch12_passmanager_and_jit.md)
- [Chapter 13: LLVM Backend: GlobalISel & MachineIR](part_4_llvm_backend/ch13_llvm_backend_deep_dive.md)
- [Chapter 14: Register Allocation & MC Layer](part_4_llvm_backend/ch14_register_allocation.md)

# Part V: Agam Compiler System Architecture
- [Chapter 15: Compiler Pipeline Walkthrough](part_5_agam_architecture/ch15_compiler_pipeline.md)
- [Chapter 16: Tensors & Algebraic Effects](part_5_agam_architecture/ch16_language_features.md)
- [Chapter 17: Incremental Daemon & Sandboxed Execution](part_5_agam_architecture/ch17_daemon_and_sandbox.md)
- [Chapter 18: Indic Grammatical Design Principles](part_5_agam_architecture/ch18_indic_grammar_rules.md)

# Part VI: The Agam Language Programming Guide
- [Chapter 19: Getting Started & Basics](part_6_language_guide/ch19_getting_started_and_basics.md)
- [Chapter 19b: Structured Concurrency & Async](part_6_language_guide/ch19b_concurrency_and_async.md)
- [Chapter 20: Control Flow & Structs](part_6_language_guide/ch20_control_flow_and_structures.md)
- [Chapter 21: Enums, Patterns & Error Handling](part_6_language_guide/ch21_enums_patterns_and_errors.md)
- [Chapter 22: First-Class Tensors & Numerical AI](part_6_language_guide/ch22_tensors_and_numerical_ai.md)
- [Chapter 23: Algebraic Effect Handlers in Depth](part_6_language_guide/ch23_algebraic_effects_in_depth.md)
- [Chapter 24: Modules & Packages](part_6_language_guide/ch24_modules_packages_and_ffi.md)
- [Chapter 24b: Security & Cryptography](part_6_language_guide/ch24b_security_and_crypto.md)
- [Chapter 24c: FFI & Cross-Language Interop](part_6_language_guide/ch24c_ffi_interop.md)
- [Chapter 25: Metaprogramming & Tooling](part_6_language_guide/ch25_metaprogramming_and_tooling.md)
- [Chapter 25b: Real-World Code Cookbook](part_6_language_guide/ch25b_cookbook.md)
- [Chapter 25c: Standard Library Reference](part_6_language_guide/ch25c_stdlib_reference.md)

# Part VII: Advanced Ecosystem & Tooling
- [Chapter 26: Diagnostic Engineering & Spans](part_7_ecosystem_and_tooling/ch26_diagnostics_and_spans.md)
- [Chapter 27: Testing Methodologies & Fuzzing](part_7_ecosystem_and_tooling/ch27_compiler_testing_and_fuzzing.md)
- [Chapter 28: Language Server Protocol (LSP)](part_7_ecosystem_and_tooling/ch28_lsp_architecture.md)
- [Chapter 29: Source Code Formatting Engine](part_7_ecosystem_and_tooling/ch29_formatter_engine.md)
- [Chapter 29b: Package Registry & Distribution](part_7_ecosystem_and_tooling/ch29b_package_registry.md)
- [Chapter 30: Cross-Compilation & Target Packs](part_7_ecosystem_and_tooling/ch30_cross_compilation_targets.md)
- [Chapter 31: Profiling & Performance Measurement](part_7_ecosystem_and_tooling/ch31_profiling_and_benchmarking.md)

# Part VIII: GPU, Hardware Acceleration & AI-Native Infrastructure
- [Chapter 32: GPU Compute Pipeline & Kernels](part_8_gpu_and_acceleration/ch32_gpu_compute_pipeline.md)
- [Chapter 33: SPIR-V Backend & Vulkan/OpenCL](part_8_gpu_and_acceleration/ch33_spirv_backend.md)
- [Chapter 34: Tile Abstractions & TMA Pipelines](part_8_gpu_and_acceleration/ch34_tile_abstractions_tma.md)
- [Chapter 35: Hardware Introspection & Auto-Tuning](part_8_gpu_and_acceleration/ch35_hardware_introspection.md)
- [Chapter 36: NPU Heterogeneous Dispatch](part_8_gpu_and_acceleration/ch36_npu_heterogeneous_dispatch.md)

# Back Matter
- [Appendix A: Comprehensive Agam Crate Map](back_matter/appendix_a_crate_map.md)
- [Appendix B: Annotated Bibliography](back_matter/appendix_b_bibliography.md)
- [Appendix C: Glossary of Technical Terms](back_matter/appendix_c_glossary.md)
- [Appendix D: Architecture Decision Records (ADRs)](back_matter/appendix_d_architecture_decisions.md)
