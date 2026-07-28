# MIR Todo Gaps

Current `todo!()` paths and semantic gaps in `agam_mir::lower` and the codegen backends, as of May 2026.

> **Why this matters:** These are the real compiler gaps blocking end-to-end compilation of non-trivial Agam programs. The build succeeds and tests pass, but these paths will panic at runtime if exercised.

## agam_mir::lower — HIR to MIR Lowering

### Struct Literals

**Location:** `crates/core/agam_mir/src/lower.rs` — `lower_expr` for `HirExprKind::StructLiteral`

**Gap:** Struct literal construction is not lowered through MIR. A struct like `Point { x: 1.0, y: 2.0 }` will hit a `todo!()`.

**Blocked by:** Phase T0-type-system (generics), Phase T0-object-model (struct layout)

### Enum Variant Construction

**Location:** `lower.rs` — `HirExprKind::EnumConstruct`

**Gap:** Enum variant construction is not lowered. `Some(x)`, `Err("message")`, and any custom enum are not emittable.

**Blocked by:** Phase T0-type-system (sum types/enums must be in the type system first)

### Match Expression Lowering

**Location:** `lower.rs` — `HirExprKind::Match`

**Gap:** `match` expressions do not lower to MIR. This means:
- No pattern matching on enums
- No exhaustiveness checking in codegen
- No switch/jump table emission in backends

**Blocked by:** Phase T0-type-system (pattern matching)

## Backend Gaps

### Enum Layout in C Backend

**Location:** `crates/core/agam_c_emitter/` — enum type emission

**Gap:** C backend does not emit discriminated union layouts for enum types. Enums passed to C-emitted code cannot be represented correctly.

### Enum Layout in LLVM Backend

**Location:** `crates/core/agam_llvm/` — enum LLVM IR type construction

**Gap:** No `llvm::Type` is generated for enum types. The LLVM backend cannot codegen any function that uses enum parameters or returns.

### Trait Method Dispatch

**Location:** Both backends

**Gap:** Dynamic dispatch via `dyn Trait` is not implemented. All dispatch is currently static (monomorphized) only, and even that requires generics to be working first.

## Sema Gaps

### Generic Type Resolution

**Location:** `crates/core/agam_sema/` — type checker

**Gap:** The type checker does not support type parameters (`T`, `<T: Bound>`, etc.). This means:
- No `Option<T>`, `Result<T, E>` in stdlib
- No generic collections
- No type inference for generic calls

### Ownership/Lifetime Tracking

**Location:** `agam_sema` — no ownership pass exists

**Gap:** There is no borrow checker, ARC insertion pass, or ownership analysis. Memory model decision made (Hybrid ARC + Value Semantics per Phase T2-memory-model) but not implemented.

## MIR Ops Present but Untested

These MIR ops exist in the surface definition but have limited test coverage:

| Op | Status |
|----|--------|
| `Switch` | Defined; `match` lowering blocked |
| `EnumConstruct` | Defined; HIR→MIR lowering blocked |
| `EnumTag` | Defined; backend emission blocked |
| `EnumPayload` | Defined; backend emission blocked |
| `GpuKernelLaunch` | Working (Phase 23) |
| `Perform` | Working (Phase 20) |

## How to Work Around These Gaps

When writing `.agam` examples or test programs that should compile today:
- **Do not use** `match`, enums, `Option`, `Result`, or generic types
- **Use** scalar types (`i32`, `f64`, `bool`), simple structs, functions, effects
- **GPU kernels** work if they stick to pointer/scalar parameters (Phase 23 validated)
- **Effects** (`perform`/`handle`) work end-to-end through both backends
