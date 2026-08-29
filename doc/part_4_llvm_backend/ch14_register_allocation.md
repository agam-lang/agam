# Chapter 14: Register Allocation Algorithms & Machine Code (MC) Layer

> **Core Literature Grounding**: *LLVM Code Generation: A Deep Dive into Compiler Backend Development* by Quentin Colombet  
> **Compiler Module Focus**: [`agam_codegen`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/backends/agam_codegen)

---

## 14.1 The Register Allocation Problem

Target CPUs possess a strictly finite number of physical registers (e.g., 16 general-purpose registers on x86_64, 31 on AArch64). However, MachineIR (MIR) instructions operate on an infinite set of **Virtual Registers** (`%vreg0`, `%vreg1`).

**Register Allocation** maps virtual registers to physical hardware registers while minimizing memory spill operations.

---

## 14.2 Register Allocation Algorithms

### 1. Graph Coloring Register Allocation (Chaitin-Briggs)
1. **Liveness Analysis**: Computes live ranges for all virtual registers.
2. **Interference Graph Construction**: Constructs a graph $G=(V, E)$ where vertices $V$ represent virtual registers and edges $E$ represent overlapping live ranges.
3. **Graph Coloring ($K$-Coloring)**: Colors the graph using $K$ physical registers.
4. **Spilling**: If the graph chromatic number exceeds $K$, virtual registers with low use intensity are spilled to stack memory (`mov [rsp+16], rax`).

### 2. Greedy Register Allocator (LLVM Production Allocator)
LLVM's production allocator processes live ranges in priority order based on execution frequency, splitting live ranges across basic block boundaries to minimize spill code overhead.

---

## 14.3 The MC (Machine Code) Layer

The **MC Layer** is LLVM's lowest level component. It converts physical `MCInst` instructions into binary object files (`.o`, `.obj` in ELF, COFF, or Mach-O format) and resolves symbol relocations (`R_X86_64_PC32`).
