# Chapter 7: High-Level & Medium-Level Intermediate Representations (HIR & MIR)

> **Core Literature Grounding**: *Engineering a Compiler* (Chapter 5) by Keith D. Cooper & Linda Torczon  
> **Compiler Module Focus**: [`agam_hir`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/middle/agam_hir), [`agam_mir`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/middle/agam_mir)

---

## 7.1 Multi-Stage Intermediate Representations

Compilers decouple language syntax from target machine optimization by introducing intermediate representations. Cooper & Torczon emphasize using intermediate forms tailored to specific compilation passes:

```text
AST (Abstract Syntax Tree)
          │
          ▼ AST Lowering
HIR (High-Level IR - `agam_hir`)
  - Preserves user types, pattern matching, algebraic effects
  - Desugars complex syntactic sugar
          │
          ▼ Desugaring & Control-Flow Lowering
MIR (Medium-Level IR - `agam_mir`)
  - Control Flow Graph (CFG) of Basic Blocks
  - Explicit temporaries (_1, _2, _3)
  - Static Single Assignment (SSA) form
```

---

## 7.2 High-Level IR (HIR - `agam_hir`)

`agam_hir` simplifies complex surface syntax while maintaining high-level type annotations and algebraic effect structures.

### Key HIR Responsibilities:
- **Desugaring Compound Control Flow**: Translating `for` loops into `while` loops or basic blocks.
- **Pattern Match Simplification**: Transforming complex nested `match` expressions into explicit decision trees.
- **Explicit Type Resolution**: Replacing inferred types with fully qualified type IDs.

---

## 7.3 Medium-Level IR (MIR - `agam_mir`)

`agam_mir` represents code as a control flow graph of basic blocks with explicit temporaries and SSA assignments.

```rust
pub struct MirFunction {
    pub name: String,
    pub params: Vec<LocalId>,
    pub return_ty: Type,
    pub basic_blocks: IndexVec<BasicBlockId, BasicBlock>,
    pub local_decls: IndexVec<LocalId, LocalDecl>,
}

pub struct BasicBlock {
    pub statements: Vec<Statement>,
    pub terminator: Terminator,
}

pub enum Statement {
    Assign(Place, Rvalue),
    StorageLive(LocalId),
    StorageDead(LocalId),
}

pub enum Terminator {
    Goto(BasicBlockId),
    Branch { cond: Operand, then_block: BasicBlockId, else_block: BasicBlockId },
    SwitchInt { discr: Operand, targets: SwitchTargets },
    Return(Operand),
    YieldEffect { effect_id: u32, payload: Operand, resume_bb: BasicBlockId },
}
```
