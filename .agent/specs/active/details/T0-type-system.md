# Phase T0-type-system � Type System Completion

**Status:** open
**Tier:** 0 (Foundation Completion)
**Priority:** Critical path — most Tier 1+ work depends on a complete type system

## Scope

Implement generics, sum types, pattern matching, type inference, and foundational utility types (Option, Result) across the full compiler pipeline. Without this, the standard library cannot provide type-safe collections, iterators, or real APIs.

## Why This Is Critical Path

- No generics = no type-safe containers = no real stdlib
- No enums/sum types = no safe error handling, no pattern matching
- No type inference = excessive annotation burden, fails the "Python readability" promise
- Every competitor (Rust, Swift, Zig, Mojo, Go) has these features

## Deliverables

### Generics (Parametric Polymorphism)
- [ ] Type parameters on functions: `fn map<T, U>(list: List<T>, f: fn(T) -> U) -> List<U>`
- [ ] Type parameters on structs: `struct Pair<A, B> { first: A, second: B }`
- [ ] Type parameters on traits: `trait Iterator<Item>`
- [ ] Monomorphization in codegen (each concrete type gets its own code)
- [ ] Trait bounds on type parameters: `fn sort<T: Ord>(list: List<T>)`
- [ ] Where clauses for complex bounds
- [ ] Parser, AST, sema, HIR, MIR, codegen all updated

### Sum Types and Enums
- [ ] Enum declarations with variants: `enum Option<T> { Some(T), None }`
- [ ] Variants with named fields: `enum Result<T, E> { Ok(T), Err(E) }`
- [ ] Simple enums without data: `enum Color { Red, Green, Blue }`
- [ ] Enum method implementation via `impl`

### Pattern Matching
- [ ] `match` expression (advance mode) / `match` statement (base mode)
- [ ] Exhaustiveness checking (all variants must be covered)
- [ ] Wildcard patterns (`_`)
- [ ] Binding patterns (`Some(x)`)
- [ ] Guard clauses (`Some(x) if x > 0`)
- [ ] Nested pattern matching
- [ ] Or-patterns (`Red | Green`)

### Type Inference
- [ ] Local type inference for `let` bindings: `let x = 42` infers `i32`
- [ ] Return type inference for closures/lambdas
- [ ] Explicit annotations required at function boundaries (Hindley-Milner local inference)
- [ ] Generic type argument inference at call sites

### Built-in Utility Types
- [ ] `Option<T>` with `Some(T)` and `None` — first-class in the stdlib
- [ ] `Result<T, E>` with `Ok(T)` and `Err(E)` — first-class in the stdlib
- [ ] `?` operator for early return on `None`/`Err` (or equivalent sugar)

### Type Aliases
- [ ] `type Name = String`
- [ ] `type Callback<T> = fn(T) -> bool`

### Const Generics (stretch)
- [ ] Integer-parameterized types: `struct Array<T, const N: usize>`
- [ ] Useful for SIMD widths, fixed-size arrays, tensor shapes

## Responsible Crates

- `agam_parser` — syntax for generics, enums, match, type aliases
- `agam_ast` — AST nodes for all new constructs
- `agam_sema` — type checking, inference, exhaustiveness checking
- `agam_hir` — typed representation of generics and enums
- `agam_mir` — monomorphized representation after generic expansion
- `agam_codegen` — emit code for monomorphized types and match dispatch
- `agam_jit` — JIT support for new constructs

## Dependencies

- Phase T0-grammar-spec (grammar spec) should ideally be written first or in parallel
- Phase T0-object-model (object model) depends on this — trait bounds need generics

## Test Strategy

- Parser tests for all new syntax forms
- Sema tests for type checking, inference, exhaustiveness
- End-to-end tests: compile and run programs using generics, enums, pattern matching
- Negative tests: type errors, non-exhaustive matches, constraint violations
