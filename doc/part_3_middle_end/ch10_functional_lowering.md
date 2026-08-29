# Chapter 10: Lowering Functional & Effectful Semantics

> **Core Literature Grounding**: *Modern Compiler Implementation in C* (Chapters 14–15) by Andrew W. Appel  
> **Compiler Module Focus**: [`agam_hir`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/middle/agam_hir), [`agam_mir`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/middle/agam_mir)

---

## 10.1 Functional to Imperative Lowering

Appel's *Modern Compiler Implementation in C* demonstrates how high-level functional concepts (closures, pattern matching, algebraic effects) are lowered into low-level imperative basic blocks.

---

## 10.2 Closure Conversion

When anonymous functions capture variables from enclosing lexical scopes, the compiler transforms them into **explicit closures**:

```text
High-Level Source:
  let factor = 10;
  let multiplier = fn(x: Int) -> Int { x * factor };

Lowered MIR Transformation:
  struct Closure_1 {
      fn_ptr: fn(*const Closure_1, i64) -> i64,
      env_factor: i64,
  }
```

Captures are explicitly stored inside environment struct payloads, converting indirect function invocations into standard C ABI calls passing the environment pointer.

---

## 10.3 Pattern Match Desugaring

Complex pattern matching (`match target { Arm1 => ..., Arm2 => ... }`) is desugared into decision trees composed of `SwitchInt` and `Branch` terminators:

```text
                     ┌──────────────────────────┐
                     |  SwitchInt(target.tag)   |
                     └────────────┬─────────────┘
                                  │
                   ┌──────────────┴──────────────┐
                   │ Tag == 0                    │ Tag == 1
                   ▼                             ▼
      ┌──────────────────────────┐  ┌──────────────────────────┐
      |  Extract Circle.radius   |  | Extract Rect.w, Rect.h   |
      |  Evaluate Arm 1          |  | Evaluate Arm 2           |
      └──────────────────────────┘  └──────────────────────────┘
```

---

## 10.4 Algebraic Effect Suspension Frames

In Agam, `perform Effect(...)` suspends execution and yields control to an enclosing handler.

During MIR lowering, `perform` operations are converted into `YieldEffect` terminators that:
1. Spill active local temporaries into a stack frame context buffer.
2. Pass the effect payload and resume basic block ID to `agam_runtime_yield`.
3. Allow the handler to resume execution at `resume_bb` upon invocation of `resume()`.
