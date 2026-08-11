# Chapter 13: LLVM Backend Architecture: SelectionDAG, GlobalISel & MachineIR

> **Core Literature Grounding**: *LLVM Code Generation: A Deep Dive into Compiler Backend Development* by Quentin Colombet  
> **Compiler Module Focus**: [`agam_codegen`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/backends/agam_codegen)

---

## 13.1 Overview of the LLVM Target Backend Architecture

Quentin Colombet's definitive work details how LLVM translates target-independent LLVM IR into physical, hardware-specific machine instructions:

```text
LLVM IR
   │
   ▼
 ┌───────────────────────────┐
 │ SelectionDAG / GlobalISel │ -> Converts Target-Independent IR to Target Nodes
 └─────────────┬─────────────┘
               │
               ▼
 ┌───────────────────────────┐
 │   MachineIR (MIR Layer)   │ -> Machine-level SSA instructions with virtual registers
 └─────────────┬─────────────┘
               │
               ▼
 ┌───────────────────────────┐
 │    Register Allocation    │ -> Maps infinite Virtual Registers -> Finite Physical Registers
 └─────────────┬─────────────┘
               │
               ▼
 ┌───────────────────────────┐
 │     MC (Machine Code)     │ -> Instruction Assembly & Binary Object Writing (.o, .obj)
 └───────────────────────────┘
```

---

## 13.2 SelectionDAG vs. GlobalISel

1. **SelectionDAG (Legacy Pipeline)**:
   - Constructs a Directed Acyclic Graph (DAG) for each Basic Block.
   - Performs **Type Legalization** (splits unsupported types like `i128` into `i64` pairs) and **DAG Combine** optimizations.
   - Translates DAG nodes into target instructions using pattern matching defined in TableGen files (`.td`).
2. **GlobalISel (Global Instruction Selection Framework)**:
   - Designed and architected by Quentin Colombet.
   - Operates globally across whole functions rather than basic blocks.
   - Operates directly on **MachineIR (MIR)** using four fast sequential passes: `IRTranslator` $\rightarrow$ `Legalizer` $\rightarrow$ `RegisterBankSelect` $\rightarrow$ `InstructionSelect`.

---

## 13.3 TableGen (`.td`) Target Descriptions

LLVM target instruction sets (x86_64, AArch64, RISC-V) are declared using the **TableGen** domain-specific language (`.td` files).

TableGen defines:
- **Register Classes**: `GR64` (`rax`, `rbx`, `rcx`), `FR64` (`xmm0`–`xmm15`).
- **Instruction Definitions**: Opcode encodings, register constraints, side effects.
- **Pattern Matching Rules**: Mapping IR operations directly to hardware opcodes.

```tablegen
// Example TableGen pattern matching 64-bit addition on x86
def ADD64rr : I<0x01, MRMDestReg, (outs GR64:$dst), (ins GR64:$src1, GR64:$src2),
                "add{q}\t{$src2, $dst|$dst, $src2}",
                [(set GR64:$dst, (add GR64:$src1, GR64:$src2))]>;
```
