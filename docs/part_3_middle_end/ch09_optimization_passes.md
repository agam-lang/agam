# Chapter 9: Middle-End Optimization Passes

> **Core Literature Grounding**: *Engineering a Compiler* (Chapters 8 & 10) by Keith D. Cooper & Linda Torczon  
> **Compiler Module Focus**: [`agam_mir::opt`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/middle/agam_mir)

---

## 9.1 The Middle-End Optimization Pipeline

The goal of middle-end optimization passes is to rewrite MIR control flow graphs into faster, smaller, and memory-efficient forms while preserving original program semantics.

```text
Unoptimized MIR
       │
       ▼  Pass 1: Dead Code Elimination (DCE)
       ▼  Pass 2: Constant Folding & Propagation
       ▼  Pass 3: Function Inlining
       ▼  Pass 4: Loop Invariant Code Motion (LICM)
       ▼
Optimized MIR -> Codegen Backend
```

---

## 9.2 Key Optimization Passes

### 1. Constant Folding & Propagation
Replaces compile-time constant expressions with evaluated literal constants and propagates definitions downstream:

$$\text{\_1 = 10 + 20} \implies \text{\_1 = 30}$$

### 2. Dead Code Elimination (DCE)
Traverses the CFG definition-use chain to eliminate instructions and basic blocks whose results are never read:

```rust
// Before DCE
_1 = Const(42); // Unused statement
_2 = Const(100);
Return(_2);

// After DCE
_2 = Const(100);
Return(_2);
```

### 3. Function Inlining
Replaces function call sites directly with the target function's basic blocks, eliminating stack frame setup overhead and unlocking scalar optimizations across caller-callee boundaries.

### 4. Loop Invariant Code Motion (LICM)
Identifies statements inside loop blocks whose operand inputs do not change across iterations, hoisting them into the loop pre-header block.
