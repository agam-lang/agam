# The LLM Wiki (Second Brain)

This directory (`.agent/wiki/`) is the persistent knowledge base for all AI agents working on the Agam-Lang repository.

## Rules for Agents

1. **Synthesize, Don't Forget:** If you make a major architectural breakthrough, discover a subtle bug in how the AST is structured, or realize an approach *does not work*, do not let it get lost in the chat history. Write a concise `.md` file in this directory detailing your findings.
2. **Query Before Starting:** When starting a complex task, check this directory (using `list_dir` or `grep_search`) for any existing knowledge related to the component you are modifying.
3. **Structured Naming:** Name files clearly (e.g., `ast-memory-model.md`, `failed-type-inference-approaches.md`).
4. **No Code Dumps:** This is for conceptual synthesis and rules of thumb, not for storing massive blocks of code. Use progressive disclosure.

## Contents

### Architecture & Pipeline

| File | Summary |
|------|---------|
| `compiler-pipeline-overview.md` | Full pipeline diagram (lexer → parser → sema → HIR → MIR → codegen), crate responsibilities, what works today, known gaps |
| `mir-todo-gaps.md` | Current `todo!()` paths in `agam_mir::lower` and backends: struct literals, enum variants, match, enum layout |

### Design Decisions

| File | Summary |
|------|---------|
| `memory-model-status.md` | Decision made: Hybrid ARC + Value Semantics. What is implemented vs. what needs building in Phase T2-memory-model |
| `ergonomics-and-security.md` | The balance between `@lang.base` optional syntax (e.g. role labels) and strict backend security/inference. |

### Roadmap & Navigation

| File | Summary |
|------|---------|
| `phase-naming-convention.md` | Explains the two naming schemes (numbered legacy vs. T{tier}-{slug} open phases), tier definitions, and naming rules for new phases |
| `dependency-map.md` | Full phase dependency graph: which phases must complete before others, tier flow diagram |
| `future-ecosystem-and-tooling.md` | Tracking gaps in ecosystem parity (packages, LSP, garbage collection, and error diagnostics). |

### External Technology Observations

| File | Summary |
|------|---------|
| `gpu-backend-strategy.md` | chipStar 1.3 observation: SPIR-V as vendor-neutral GPU IR for `@gpu` blocks |
| `gpu-auto-tuning-and-tiles.md` | CUDA 13.3 CompileIQ and CUDA Tile observation: genetic auto-tuner and `Tile<T,N>` single-thread illusion |
| `tensor-core-lowering.md` | OpenCL cooperative matrix extension: vendor-neutral tensor core access via SPIR-V `OpCooperativeMatrixMulAddKHR` |
| `ebpf-kernel-strategy.md` | KernelScript observation: unified eBPF kernel + userspace in a single `.agam` file via `@ebpf` annotation |
| `pqc-formal-verification.md` | Apple PQC open-source observation: ML-KEM/ML-DSA for `agam-crypto`, formal verification for crypto primitives |
