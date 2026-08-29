# Appendix D: Architecture Decision Records (ADRs)

> **Document Scope**: Foundational Architectural Decisions in Agam Language & Compiler Design

---

## ADR-001: Multi-Level Intermediate Representation Architecture (HIR & MIR)

### Status: Accepted & Implemented
### Date: 2026-01-15
### Context
Compilers bridging high-level languages (with pattern matching, algebraic effects, and multi-dimensional tensors) directly to low-level backends (such as LLVM IR or SPIR-V) encounter severe semantic gap challenges. A single IR cannot cleanly represent both functional desugaring and low-level control flow graphs with SSA $\phi$-nodes.

### Decision
Agam implements a two-tier intermediate representation pipeline:
1. **High-Level IR (`agam_hir`)**: Preserves lexical scopes, structured pattern matching decision trees, and un-lowered algebraic effect boundaries.
2. **Medium-Level IR (`agam_mir`)**: A flat, basic-block-based SSA representation with explicit CFG edges, $\phi$-nodes, and optimization passes (SCCP, GVN, DCE, Inlining, LICM).

### Consequences
- **Positive**: Clean separation of concerns; pattern matching desugaring is completely decoupled from register-friendly SSA optimizations.
- **Positive**: Frontends and macro systems target HIR without needing to understand basic block splitting or dominance frontiers.
- **Trade-off**: Requires serialization/deserialization and traversal overhead between HIR and MIR lowering passes.

---

## ADR-002: Dual-Backend Strategy (LLVM Native + Portable C11)

### Status: Accepted & Implemented
### Date: 2026-02-01
### Context
While LLVM provides world-class optimization and native code generation for major desktop and server architectures, it introduces heavy dependency footprints, complex build toolchains, and limited support for exotic or legacy embedded microcontrollers (e.g., bare-metal 16/32-bit DSPs).

### Decision
`agam_codegen` adopts a dual-backend emission strategy:
1. **Primary Backend**: Direct LLVM IR text and bitcode emitter with modern LLVM PassManager integration for high-performance native execution on x86_64, AArch64, and WebAssembly.
2. **Secondary Portable Backend**: Clean ANSI C11 source code emitter (`agam_codegen::c_emitter`) enabling universal compilation on any platform with an existing ISO C compiler.

### Consequences
- **Positive**: 100% platform reach from day one, including bare-metal microcontrollers without LLVM targets.
- **Positive**: Unlocks rapid bootstrapping and cross-compilation with zero external C++ library dependencies.
- **Trade-off**: Requires maintaining parity between LLVM IR lowering rules and C11 code generation constructs.

---

## ADR-003: Vendor-Neutral SPIR-V as Primary GPU Compute Target

### Status: Accepted & Implemented
### Date: 2026-03-10
### Context
GPU acceleration in existing systems is heavily fragmented. Direct CUDA bindings lock developers to NVIDIA hardware, while separate OpenCL C or Metal Shading Language files fragment the codebase and complicate the developer experience.

### Decision
Agam adopts Khronos **SPIR-V 1.5** binary emission (`agam_codegen::spirv`) as its primary, vendor-neutral GPU compilation target. Kernel functions annotated with `@gpu` are compiled to SPIR-V modules with `SPV_KHR_cooperative_matrix` extensions for Tensor Core / Matrix Core acceleration. Secondary NVPTX and Metal adapters are provided for vendor-specific platform tuning.

### Consequences
- **Positive**: Write-once, run-anywhere GPU kernels compatible with Vulkan Compute, OpenCL 2.0+, and Intel Level Zero across NVIDIA, AMD, Intel, Qualcomm, and Apple GPUs.
- **Positive**: GPU kernels share the same type system, syntax, and tensor semantics as host code.
- **Trade-off**: Advanced vendor-proprietary hardware features (like NVIDIA TMA) require custom emitter extensions alongside standard SPIR-V.

---

## ADR-004: Stackless State Machine Lowering for Algebraic Effects

### Status: Accepted & Implemented
### Date: 2026-04-12
### Context
Algebraic effects and handlers provide modular control flow for I/O, logging, state management, and exception handling. Traditional implementations often rely on delimited continuations with full stack copying (e.g., in OCaml 5 or Eff), which introduces runtime overhead and complicates C ABI integration.

### Decision
Agam compiles algebraic effect handlers and resumptions into **stackless state machines** at the MIR level, mirroring the compilation strategy used for async/await coroutines. Functions performing effects allocate lightweight continuation frames and transition through deterministic state machine states.

### Consequences
- **Positive**: Zero-cost abstraction for pure computations; minimal overhead on effect suspension and resumption.
- **Positive**: Clean interoperability with standard C stack frames and native debuggers (DWARF).
- **Trade-off**: Prohibits unrestricted multi-shot continuations (effects can only be resumed once per invocation in the baseline model).

---

## ADR-005: Indic Grammatical Formalism as Semantic Architecture

### Status: Accepted & Implemented
### Date: 2026-05-01
### Context
Most modern programming languages draw syntactic paradigms exclusively from Western grammatical structures (e.g., English subject-verb-object). Classical Indic linguistics, specifically Pāṇini's *Aṣṭādhyāyī* and the Tamil *Tolkāppiyam*, developed the world's most formal generative rewrite rules, semantic role formalisms (*Kāraka*), and morphophonemic composition rules (*Sandhi*).

### Decision
Agam incorporates Indic linguistic design patterns directly into its formal compiler architecture:
1. **Pāṇinian Specificity (Apavāda Override)**: Formal rewrite priority where specific rules automatically supersede general rules in pattern matching and macro evaluation.
2. **Kāraka Semantic Roles**: Parameter binding roles formalizing Agent, Object, Instrument, and Locus in function signatures.
3. **Type Sandhi**: Formal algebraic rules governing type composition, union simplification, and widening.
4. **Nyāya Diagnostic Model**: Error messages structured according to classical 4-part epistemological proofs (Thesis, Reason, Example, Application).

### Consequences
- **Positive**: Uniquely rigorous formal foundation for type theory and pattern dispatch.
- **Positive**: World's first industrial compiler directly honoring the oldest formal linguistic traditions of humanity.
