# T0-type-system: Foundation Type System Completion

Implement generics, sum types, pattern matching, type inference improvements, and foundational utility types (Option, Result) across the full compiler pipeline. This is the single highest-leverage foundation gap — no real stdlib, no safe error handling, no collections without this.

## Current State Assessment

The codebase already has partial infrastructure across all layers:

| Layer | What Exists | What's Missing |
|-------|------------|----------------|
| **AST** | `GenericParam`, `EnumDecl`, `EnumVariant`, `VariantFields`, `TraitDecl`, `ImplDecl`, `TypeAlias` nodes all present | Complete ✅ |
| **Parser** | `parse_generic_params()` already called for fn/struct/enum/trait/impl/type-alias | Needs generic type argument parsing on call-site types (e.g. `Vec<i32>`) |
| **Sema Resolver** | Registers generic names as `TypeParam` symbols, registers enum variants with `EnumVariantInfo` | No substitution binding during type resolution, no trait-bound checking |
| **Sema Checker** | Has `apply_substitution` for generic functions, basic enum path expression typing, basic exhaustiveness checking | Incomplete — generic call-site instantiation doesn't fully unify, `Option`/`Result` not registered |
| **Sema Inference** | `InferenceEngine` with Var/unification exists | Doesn't handle generic constraints or trait bounds |
| **HIR Lowering** | Resolves GPU configs, target profiles, effects | No generic metadata on HirFunction, no monomorphization prep, enum lowering partial |
| **MIR** | `monomorphize.rs` with `MonomorphKey` and name mangling exists | **Critical TODO**: cloning+substitution of generic function MIR not implemented |
| **Codegen (C/LLVM)** | Enum tag/construct/payload ops partially handled | No generic specialization output, no `Option`/`Result` codegen |
| **Tests** | 830 passing | No generic end-to-end tests |

## Proposed Changes — 4-Phase Approach

### Phase A: Generics End-to-End (Highest Priority)

> **IMPORTANT**: This is the critical path. Nothing else works without generic functions compiling end-to-end.

---

#### [MODIFY] checker.rs (`crates/middle/agam_sema/src/checker.rs`)

- Wire up generic type parameter scoping: when checking a generic function, push type params into scope as `SymbolKind::TypeParam` so the body can reference `T`
- Enhance `infer_expr` for `ExprKind::Call` to perform generic instantiation — when calling a generic function, infer type arguments from the concrete argument types
- Add generic type argument unification in the inference engine: `fn identity<T>(x: T) -> T` called with `identity(42)` should unify `T = i32`

#### [MODIFY] infer.rs (`crates/middle/agam_sema/src/infer.rs`)

- Extend `apply_substitution` to handle nested generics (`Vec<T>` → `Vec<i32>`)
- Add `instantiate_generic()` method that takes a generic function signature and concrete type args, returns a fully substituted signature

#### [MODIFY] lower.rs (`crates/middle/agam_hir/src/lower.rs`)

- Propagate generic type parameters from AST `FunctionDecl.generics` onto `HirFunction`
- Resolve generic type expressions during HIR type lowering — `T` in a generic body maps to a `TypeParam` HIR type

#### [MODIFY] nodes.rs (`crates/middle/agam_hir/src/nodes.rs`)

- Add `generics: Vec<String>` field to `HirFunction` for carrying type parameter names through to MIR

#### [MODIFY] monomorphize.rs (`crates/middle/agam_mir/src/monomorphize.rs`)

- Complete the TODO at line 133: clone the generic function's MIR body, walk all instructions, substitute `TypeParam` references with concrete types
- Generate specialized `MirFunction` instances with mangled names
- Update `Op::Call` references in all functions to use mangled names for specialized callees

#### [MODIFY] ir.rs (`crates/middle/agam_mir/src/ir.rs`)

- Add `generics: Vec<String>` field to `MirFunction` to propagate type parameter names (needed for monomorphization matching)

---

### Phase B: Enum/Sum Types Completion

#### [MODIFY] lower.rs (HIR) (`crates/middle/agam_hir/src/lower.rs`)

- Complete enum variant constructor lowering: `Some(42)` → `HirExprKind::EnumConstruct`
- Complete `match` arm pattern lowering for enum destructuring: `Some(x)` → extract payload

#### [MODIFY] lower.rs (MIR) (`crates/middle/agam_mir/src/lower.rs`)

- MIR `Switch` + `EnumTag` + `EnumPayload` ops already exist — wire up the HIR enum patterns to use them properly

#### [MODIFY] c_emitter.rs (`crates/backends/agam_codegen/src/c_emitter.rs`)

- Complete enum layout emission: tagged union struct for each enum type
- Complete `EnumConstruct`, `EnumTag`, `EnumPayload` C codegen for pattern match dispatch

#### [MODIFY] llvm_emitter.rs (`crates/backends/agam_codegen/src/llvm_emitter.rs`)

- Complete enum layout as LLVM struct types (tag + union payload)
- Complete `EnumConstruct`, `EnumTag`, `EnumPayload` IR emission

---

### Phase C: Option/Result + ? Operator

#### [MODIFY] resolver.rs (`crates/middle/agam_sema/src/resolver.rs`)

- Register `Option<T>` and `Result<T, E>` as builtin generic enum types with known variant structure
- Register `Some`, `None`, `Ok`, `Err` as variant constructors in the global scope

#### [MODIFY] checker.rs (`crates/middle/agam_sema/src/checker.rs`)

- Handle `ExprKind::Try` (the `?` operator): infer the inner type as `Result<T, E>` or `Option<T>`, produce `T` as the result type, generate early return on error/none

#### [MODIFY] exhaustive.rs (`crates/middle/agam_sema/src/exhaustive.rs`)

- Enhance exhaustiveness checking to understand `Option` and `Result` variant shapes so that `match` on these types enforces coverage

---

### Phase D: Local Type Inference Improvements

#### [MODIFY] infer.rs (`crates/middle/agam_sema/src/infer.rs`)

- Improve unification to handle bidirectional inference: `let x = 42` should default to `i32`, `let x = 3.14` to `f64`
- Add generic type argument inference at call sites: `map(list, fn)` should infer `<T, U>` from context
- Numeric literal defaulting: integer literals default to `i32`, float literals to `f64` when no other constraint exists

---

## Open Questions

> **IMPORTANT — Monomorphization strategy**: Should we monomorphize at the MIR level (like Rust) or at codegen (like C++ templates)? MIR-level is already partially implemented and gives us optimization opportunities post-specialization. **Recommendation: continue with MIR-level monomorphization.**

> **IMPORTANT — Type parameter syntax**: The parser already handles `<T>` angle brackets for generics. Should we also support `[T]` brackets as an alternative syntax, or keep `<T>` exclusively? **Recommendation: `<T>` only for simplicity and familiarity.**

## Verification Plan

### Automated Tests
```powershell
cargo check --manifest-path agam/Cargo.toml
cargo test  --manifest-path agam/Cargo.toml
```

### Specific Test Targets
- **Phase A**: Generic function parsing, type checking, HIR lowering, MIR monomorphization, codegen output
- **Phase B**: Enum construction, pattern matching, exhaustiveness in match
- **Phase C**: Option/Result type checking, `?` operator lowering, exhaustiveness for builtin enums
- **Phase D**: Inference engine improvements, numeric defaulting

### End-to-End Validation
- Compile and run: `fn identity<T>(x: T) -> T { x }; fn main() { let y = identity(42); }`
- Compile and run: `enum Option<T> { Some(T), None }; fn main() { let x = Some(42); match x { Some(v) => println(v), None => println(0) } }`
