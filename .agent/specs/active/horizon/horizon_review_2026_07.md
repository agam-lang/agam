# Agam-Lang Stream 2 Horizon Review — July 2026 Synthesis

> **Stream 2 Target**: Frontier AI/Compiler Synthesis, Specification Sync & Industry Horizon Review.

---

## 1. Executive Summary

This Horizon Review synthesizes current compiler engineering progress across Agam's 27 workspace crates and aligns upcoming development with 2026 frontier AI compiler advancements (e.g. MiniTriton kernel lowering, Kimi K3 long-context LLM agent protocols, and autonomous property fuzzing).

---

## 2. Frontier Technology Synthesis

### 2.1 MiniTriton & NVPTX Kernel Lowering
- **Observation**: Industry GPU compilers are shifting toward lightweight tensor-kernel abstractions that bypass heavy C++ dependencies.
- **Agam Alignment**: Agam's `agam_codegen` NVPTX backend already features pre-allocated buffer emitters, shared memory layout helpers (`addrspace(3)`), and CUDA driver API linkage. Next step: introduce direct kernel fusion lowering in MIR.

### 2.2 Agentic Protocol Standardization (`agamc mcp serve`)
- **Observation**: Frontier LLM agents (Kimi K3, Gemini 3.6, Claude 3.5) interact natively with build systems via the Model Context Protocol (MCP).
- **Agam Alignment**: Phase `T1-compiler-agent-tool` will extend `agam_lsp` and `agam_driver` to expose structured MCP tools (`agamc mcp serve`), SARIF diagnostic streaming, and AST-level semantic refactoring.

### 2.3 Autonomous Fuzzing & Telemetry Loops
- **Observation**: High-reliability compiler design requires continuous property-based fuzzing and telemetry collection.
- **Agam Alignment**: Integrated autonomous fuzzing loops directly into Stream 0 rules (`.agent/rules/trigger-keywords.md`).

---

## 3. Active Tier Progress & Roadmap Alignment

```
Tier 0 (Foundation: T0-type-system, T0-stdlib-io) [IN PROGRESS]
  └→ Tier 1 (DX: T1-error-messages, T1-sdk-distribution) [READY]
       └→ Tier 2 (Runtime & Security: T2-effects-runtime) [PLANNED]
```

### Current Status:
- **`T0-type-system`**: TypeStore interned Option/Result constructors, C tagged-union layouts for enum variants, local variable lexical shadowing precedence over enum constructors, and JIT specialization guard integration completed.
- **Multi-Backend Matrix**: Clean build and test pass across all 27 workspace crates (LLVM IR, NVPTX CUDA, C Emitter, Cranelift JIT).

---

## 4. Strategic Next Actions

1. **Complete `T0-type-system`**: Lower `HirExprKind::Match` and `EnumConstruct` into complete MIR control-flow graphs with scalar payload extraction across all backends.
2. **Execute `T1-error-messages`**: Upgrade diagnostic error reporting with rich source-span highlights using `miette` and `codespan-reporting`.
3. **Continuous Stream 0 Verification**: Maintain 100% test pass rate and automated benchmark performance guard.
