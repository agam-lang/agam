# Chapter 9: Middle-End Optimization Passes

> **Core Literature Grounding**: *Engineering a Compiler* (Chapters 8 & 10) by Keith D. Cooper & Linda Torczon  
> **Compiler Module Focus**: [`agam_mir::opt`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/middle/agam_mir)

---

## 9.1 The Middle-End Optimization Pipeline

The goal of middle-end optimization passes is to rewrite MIR control flow graphs into faster, smaller, and memory-efficient forms while preserving original program semantics. Each pass operates on the SSA-form MIR and produces a transformed MIR that is strictly semantically equivalent to the input.

The Agam compiler applies passes in a fixed canonical order, chosen to maximize the opportunities each pass creates for subsequent passes:

```text
Unoptimized MIR (SSA Form)
       │
       ▼  Pass 1: Sparse Conditional Constant Propagation (SCCP)
       ▼  Pass 2: Global Value Numbering (GVN)
       ▼  Pass 3: Dead Code Elimination (DCE)
       ▼  Pass 4: Function Inlining
       ▼  Pass 5: Loop Invariant Code Motion (LICM)
       ▼  Pass 6: Strength Reduction
       ▼  Pass 7: Loop Unrolling
       ▼  Pass 8: Tail Call Optimization (TCO)
       │
       ▼
Optimized MIR → Codegen Backend
```

The pass manager (`agam_mir::opt::PassManager`) coordinates iteration. Some passes are run in a **fixed-point loop** — if inlining exposes new constant-folding opportunities, the pipeline re-runs SCCP and DCE until no further changes are detected.

---

## 9.2 Sparse Conditional Constant Propagation (SCCP)

SCCP simultaneously discovers unreachable basic blocks *and* propagates compile-time constants through the CFG. Unlike simple constant folding, SCCP uses a **lattice-based abstract interpretation** over SSA values:

```text
Lattice Values:
    ⊤  (Top / Unknown)
    │
  Const(v)  (Known constant value)
    │
    ⊥  (Bottom / Overdefined — multiple reaching values)
```

**Algorithm sketch:**
1. Initialize all SSA values to `⊤` and all CFG edges to *not executable*.
2. Mark the entry block's incoming edge as *executable*.
3. For each newly-executable instruction, evaluate using lattice meet rules:
   - `Const(a) + Const(b)` → `Const(a + b)`
   - `Const(a) + ⊥` → `⊥`
   - `⊤ + anything` → `⊤` (wait for more information)
4. For conditional branches on `Const(true)`, mark only the true edge as executable.
5. Iterate until the worklist empties.

```rust
// Before SCCP
_1 = Const(10);
_2 = Const(20);
_3 = Add(_1, _2);        // Can be folded
_4 = Mul(_3, Const(2));   // Can be folded transitively
Branch(Eq(_3, Const(30)), bb_true, bb_false);

// After SCCP
_3 = Const(30);           // Folded: 10 + 20
_4 = Const(60);           // Folded: 30 * 2
Goto(bb_true);            // Branch resolved: 30 == 30 is always true
// bb_false is now unreachable and removed
```

**Literature reference:** Cooper & Torczon, §10.7 — Sparse Conditional Constant Propagation.

---

## 9.3 Global Value Numbering (GVN)

GVN detects and eliminates **redundant computations** across basic block boundaries by assigning a unique *value number* to each expression. Two expressions with identical operators and identically-numbered operands receive the same value number, and the redundant computation is replaced with a reference to the first.

```rust
// Before GVN
bb0:
  _1 = Load(ptr_x);
  _2 = Add(_1, Const(5));

bb1:                       // Dominated by bb0
  _3 = Load(ptr_x);       // Redundant load (no store to ptr_x between)
  _4 = Add(_3, Const(5)); // Redundant: same value as _2

// After GVN
bb0:
  _1 = Load(ptr_x);       // Value number: v1
  _2 = Add(_1, Const(5)); // Value number: v2

bb1:
  // _3 eliminated, replaced by _1
  // _4 eliminated, replaced by _2
  Use(_2);                 // Direct reference to v2
```

GVN requires **dominator tree traversal** — a computation in block B can only be replaced by one in block A if A dominates B (every path from entry to B passes through A).

---

## 9.4 Dead Code Elimination (DCE)

DCE traverses the CFG definition-use chain to eliminate instructions whose results are never consumed and basic blocks that are unreachable:

**Mark-sweep algorithm:**
1. **Mark phase:** Starting from all *critical* instructions (returns, stores, calls with side effects), walk backwards through def-use chains marking every instruction that contributes to a critical result.
2. **Sweep phase:** Remove all unmarked instructions. Remove basic blocks with no executable incoming edges.

```rust
// Before DCE
_1 = Const(42);       // ← Not used by any critical instruction
_2 = Const(100);
_3 = Add(_2, Const(1));
Return(_3);

// After DCE
_2 = Const(100);
_3 = Add(_2, Const(1));
Return(_3);
// _1 removed: dead definition
```

**Key subtlety:** Function calls with potential side effects are *always* marked critical, even if their return value is unused. Pure functions annotated with `@pure` can be eliminated if their result is dead.

---

## 9.5 Function Inlining

Inlining replaces a function call site with the callee's body, eliminating call overhead and exposing the callee's internals to the caller's optimization context. The inliner uses a **cost model** to decide which call sites to inline:

**Inlining heuristics:**
| Factor | Decision |
| :--- | :--- |
| Callee body ≤ 30 MIR instructions | Always inline |
| Callee called exactly once (unique call site) | Always inline |
| Call site is inside a hot loop | Inline with 3× cost budget |
| Callee is recursive | Never inline (prevents infinite expansion) |
| Callee body > 200 MIR instructions | Never inline (code size explosion) |
| `@inline` annotation on callee | Force inline regardless of size |
| `@noinline` annotation on callee | Never inline |

**Mechanics:**
1. Clone the callee's basic blocks into the caller's CFG.
2. Replace callee parameter references with the caller's argument SSA values.
3. Replace the callee's `Return(val)` terminators with assignments to the call's result SSA value, followed by a `Goto` to the block after the original call site.
4. Rename all SSA values in the inlined body to avoid conflicts.

---

## 9.6 Loop Invariant Code Motion (LICM)

LICM identifies statements inside loop blocks whose operand inputs do not change across iterations and hoists them into the loop **pre-header** block — a dedicated block inserted before the loop header that executes exactly once.

```rust
// Before LICM
bb_preheader:
  Goto(bb_loop);

bb_loop:                         // Loop header
  _i = Phi(Const(0), _i_next);
  _inv = Mul(Const(4), _stride); // ← Loop-invariant! _stride doesn't change
  _addr = Add(_base, _inv);
  _val = Load(_addr);
  _i_next = Add(_i, Const(1));
  Branch(Lt(_i_next, _n), bb_loop, bb_exit);

// After LICM
bb_preheader:
  _inv = Mul(Const(4), _stride); // ← Hoisted out of loop
  Goto(bb_loop);

bb_loop:
  _i = Phi(Const(0), _i_next);
  _addr = Add(_base, _inv);     // Uses hoisted value
  _val = Load(_addr);
  _i_next = Add(_i, Const(1));
  Branch(Lt(_i_next, _n), bb_loop, bb_exit);
```

**Safety condition:** An instruction can be hoisted if (a) all its operands are defined outside the loop or are themselves loop-invariant, and (b) the instruction has no side effects that depend on iteration order.

---

## 9.7 Strength Reduction

Strength reduction replaces expensive operations with cheaper equivalents, particularly inside loops where an induction variable's value follows an arithmetic progression:

| Before | After | Savings |
| :--- | :--- | :--- |
| `Mul(_i, Const(8))` | `_acc += 8` per iteration | Multiply → Add |
| `Div(_x, Const(16))` | `Shr(_x, Const(4))` | Division → Shift |
| `Rem(_x, Const(8))` | `And(_x, Const(7))` | Modulo → Bitwise AND |
| `Mul(_x, Const(15))` | `Sub(Shl(_x, 4), _x)` | Multiply → Shift+Sub |

**Induction variable recognition:** For a loop counter `_i` incremented by constant stride `s`, any expression `_i * c` inside the loop body is replaced with an accumulator `_acc` initialized to `start * c` and incremented by `s * c` each iteration.

---

## 9.8 Loop Unrolling

Loop unrolling replicates the loop body multiple times to reduce branch overhead and enable instruction-level parallelism. Agam supports both **full unrolling** (for loops with compile-time-known trip counts ≤ 16) and **partial unrolling** (replicating the body N times with a remainder loop):

```rust
// Before unrolling (trip count = 4, known at compile time)
for i in 0..4 { sum += arr[i]; }

// After full unrolling
sum += arr[0];
sum += arr[1];
sum += arr[2];
sum += arr[3];
```

For partial unrolling with factor 4 on a dynamic trip count:
```rust
// Unrolled body (handles 4 iterations per pass)
while i + 4 <= n {
    sum += arr[i]; sum += arr[i+1]; sum += arr[i+2]; sum += arr[i+3];
    i += 4;
}
// Remainder loop
while i < n { sum += arr[i]; i += 1; }
```

---

## 9.9 Tail Call Optimization (TCO)

When a function's final action is a call to another function (or itself), TCO reuses the current stack frame instead of allocating a new one. This transforms recursive algorithms into constant-stack iterative loops:

```agam
// Agam source — tail-recursive factorial
fn factorial(n: Int, acc: Int) -> Int {
    if n <= 1 { return acc; }
    return factorial(n - 1, n * acc);  // Tail position
}
```

The MIR optimizer detects the tail call pattern and rewrites it as:
```rust
// After TCO — transformed to loop
bb_entry:
  _n = Arg(0); _acc = Arg(1);
bb_loop:
  Branch(Le(_n, Const(1)), bb_return, bb_recurse);
bb_recurse:
  _acc_new = Mul(_n, _acc);
  _n_new = Sub(_n, Const(1));
  _n = _n_new; _acc = _acc_new;
  Goto(bb_loop);                // No stack growth!
bb_return:
  Return(_acc);
```

**Literature reference:** Appel, §7.3 — Tail Calls and Continuation Passing Style.
