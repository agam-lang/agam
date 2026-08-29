# Chapter 8: Control Flow Graphs & Static Single Assignment (SSA) Form

> **Core Literature Grounding**: *Engineering a Compiler* (Chapter 9) by Keith D. Cooper & Linda Torczon  
> **Compiler Module Focus**: [`agam_mir`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/middle/agam_mir)

---

## 8.1 Control Flow Graph (CFG) Construction

A **Control Flow Graph (CFG)** is a directed graph $G = (V, E)$ where vertices $V$ represent Basic Blocks and edges $E$ represent control flow jumps (`Goto`, `Branch`, `SwitchInt`).

```text
                     ┌───────────────────────┐
                     |  BasicBlock 0 (Entry) |
                     |  _1 = Const(10)       |
                     |  _2 = _1 > 5          |
                     |  Branch(_2, BB1, BB2) |
                     └───────────┬───────────┘
                                 │
                   ┌─────────────┴─────────────┐
                   │                           │
                   ▼                           ▼
      ┌───────────────────────┐   ┌───────────────────────┐
      |  BasicBlock 1 (Then)  |   |  BasicBlock 2 (Else)  |
      |  _3 = Const(100)      |   |  _3 = Const(200)      |
      |  Goto(BB3)            |   |  Goto(BB3)            |
      └────────────┬──────────┘   └────────────┬──────────┘
                   │                           │
                   └─────────────┬─────────────┘
                                 │
                                 ▼
                    ┌─────────────────────────┐
                    |  BasicBlock 3 (Exit)    |
                    |  _4 = Phi(BB1:_3, BB2:_3|
                    |  Return(_4)             |
                    └─────────────────────────┘
```

---

## 8.2 The SSA Property & $\phi$-Nodes

In **Static Single Assignment (SSA)** form:
1. Every temporary variable is defined exactly once.
2. Every use of a variable is dominated by its definition point.

### The $\phi$-Node (Phi Function)
When control flow branches merge at a join point, values defined in separate predecessor blocks are reconciled using a $\phi$-node:

$$\text{\_4} = \phi(\text{BB1: \_3}, \text{BB2: \_3})$$

---

## 8.3 Dominance & Dominance Frontiers

Computing minimal SSA form requires dominance analysis over the CFG graph:

### 1. Dominance Definition
A basic block $D$ dominates block $B$ ($D \text{ dom } B$) if every path from the entry block $BB_0$ to $B$ must pass through $D$.

### 2. Dominance Frontier ($DF$)
The Dominance Frontier of a block $X$ is the set of all nodes $Y$ such that $X$ dominates a predecessor of $Y$, but does not strictly dominate $Y$ itself:

$$DF(X) = \{ Y \mid \exists P \in \text{Pred}(Y) \text{ s.t. } X \text{ dom } P \text{ and } X \text{ does not strictly dom } Y \}$$

$\phi$-nodes are placed at the iterated dominance frontier $DF^+(B)$ for all basic blocks $B$ containing variable assignments.
