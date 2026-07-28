# T0-type-system Task Checklist

## Phase A: Generics End-to-End
- [x] A1: Add `generics: Vec<String>` to `HirFunction` in `nodes.rs` (pre-existing)
- [x] A2: Add `generics: Vec<String>` to `MirFunction` in `ir.rs`
- [x] A3: Propagate generics in HIR lowering (`lower.rs`) (pre-existing)
- [x] A4: Propagate generics from HIR to MIR lowering
- [ ] A5: Enhance checker to scope type params during generic function checking
- [x] A6: Complete monomorphization: clone + substitute generic MIR functions
- [x] A7: Rewrite call sites to use mangled names post-monomorphization
- [x] A8: Add tests for generic functions end-to-end (monomorphize tests)
- [x] A9: Verify `cargo check` and `cargo test` pass (835 tests, 0 failures)

## Phase B: Enum/Sum Types Completion
- [ ] B1: Complete enum variant constructor lowering in HIR
- [ ] B2: Complete match arm pattern lowering for enum destructuring in HIR
- [ ] B3: Wire HIR enum patterns into MIR Switch/EnumTag/EnumPayload
- [ ] B4: Complete enum layout + codegen in C emitter
- [ ] B5: Complete enum layout + codegen in LLVM emitter
- [ ] B6: Add tests for enum construction and pattern matching
- [ ] B7: Verify `cargo check` and `cargo test` pass

## Phase C: Option/Result + ? Operator
- [ ] C1: Register Option<T> and Result<T, E> as builtin enum types
- [ ] C2: Register Some/None/Ok/Err as variant constructors
- [ ] C3: Implement ? operator typing in checker
- [ ] C4: Enhance exhaustiveness checking for Option/Result
- [ ] C5: Add tests
- [ ] C6: Verify

## Phase D: Local Type Inference Improvements
- [ ] D1: Numeric literal defaulting (i32/f64)
- [ ] D2: Generic type argument inference at call sites
- [ ] D3: Add tests
- [ ] D4: Verify
