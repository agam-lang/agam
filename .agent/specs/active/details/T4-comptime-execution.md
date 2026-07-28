# Phase T4-comptime-execution � Compile-Time Execution

**Status:** open
**Tier:** 4 (Performance and Optimization Depth)

## Scope

Add compile-time function evaluation, enabling constant folding of arbitrary expressions, compile-time code generation, type-level computation, and static assertions. Zig's `comptime` is a major differentiator; Agam should have an equivalent.

## Deliverables

### Compile-Time Evaluation
- [ ] `comptime` keyword (or `const fn`) for functions evaluable at compile time
- [ ] Compile-time evaluation of constant expressions
- [ ] Compile-time string manipulation (useful for code generation)
- [ ] Compile-time array/collection construction

### Conditional Compilation
- [ ] `comptime if` for compile-time branching
- [ ] Platform-conditional code without preprocessor macros
- [ ] Feature flags resolved at compile time

### Static Assertions
- [ ] `comptime assert(condition, "message")` — fails compilation if false
- [ ] Type-level assertions (e.g., size constraints, trait bounds)

### Type-Level Computation
- [ ] Compute types at compile time based on input types
- [ ] Useful for: tensor shape validation, SIMD width selection, buffer sizing

## Responsible Crates

- `agam_parser` — `comptime` keyword
- `agam_sema` — compile-time evaluation engine (MIR interpreter)
- `agam_mir` — constant folding of comptime results

## Dependencies

- Phase T0-type-system (type system) — comptime generics interaction
- Phase T4-incremental-compile/O4 — comptime enables advanced optimization patterns
