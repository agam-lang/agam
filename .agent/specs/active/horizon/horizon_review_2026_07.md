# Agam-Lang Stream 2 Horizon Review — July 2026 Synthesis

> **Stream 2 Target**: Frontier AI/Compiler Synthesis, Mathematical Algorithm Integration, & Specification Sync.

---

## 1. Executive Summary

This Horizon Review synthesizes current compiler engineering progress across Agam's 27 workspace crates and aligns upcoming development with frontier mathematical algorithms and formal compiler principles (e.g., commuting square-zero algebra for tensor kernel fusion, AST-level Model Context Protocol streaming, Nyāya 4-part proof diagnostics, and autonomous property fuzzing).

---

## 2. Frontier Algorithm & Mathematical Synthesis

### 2.1 Direct MIR Tensor Kernel Fusion via Commuting Square-Zero Algebra
- **Working Principle**: E-graph equality saturation combined with nilpotent term-rewriting ($\mathcal{S} = \mathbb{F}_2[z_1, \dots, z_r]/(z_i^2)$) to eliminate redundant tensor memory strides and fuse multi-head attention loops into zero-cost assembly.
- **Agam Alignment**: Agam's `agam_codegen` universal GPU backend features pre-allocated buffer emitters and shared memory layout helpers (`addrspace(3)`). Next step: introduce direct square-zero term-rewriting pass in `agam_mir`.

### 2.2 AST-Level Protocol Standardization (`agamc mcp serve`)
- **Working Principle**: Direct AST symbol graph inspection and streamable diagnostic serialization over the Model Context Protocol (MCP).
- **Agam Alignment**: Phase `T1-compiler-agent-tool` extends `agam_lsp` and `agam_driver` to expose structured MCP tools (`agamc mcp serve`), Nyāya SARIF proof streaming, and AST-level semantic refactoring for AI agent collaboration.

### 2.3 Autonomous Fuzzing & Telemetry Loops
- **Working Principle**: Continuous property-based AST/Sema fuzzing and invariant telemetry collection.
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
- **Multi-Backend Matrix**: Clean build and test pass across all 27 workspace crates (LLVM IR, Universal GPU Emitter, C Emitter, Cranelift JIT).

---

## 4. Strategic Next Actions

1. **Complete `T0-type-system`**: Lower `HirExprKind::Match` and `EnumConstruct` into complete MIR control-flow graphs with scalar payload extraction across all backends.
2. **Execute `T1-error-messages`**: Upgrade `agam_errors` to produce native **Nyāya 4-part proof diagnostics** (*Fact, Reason, Fix, Law*).
3. **Continuous Stream 0 Verification**: Maintain 100% test pass rate and automated benchmark performance guard.
