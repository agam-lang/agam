# Phase T6-formal-verification � Formal Verification Integration

**Status:** open
**Tier:** 6 (Frontier and Long-Horizon)

## Scope

Integrate SMT solver-based formal verification for safety-critical code paths. Extend the existing `agam_smt` crate into a real verification tool.

## Deliverables

- [ ] SMT solver integration (Z3 or CVC5) for proof obligations
- [ ] `@verify` annotation for functions with pre/post-conditions
- [ ] `requires(condition)` and `ensures(condition)` contract syntax
- [ ] Loop invariant inference (or manual annotation)
- [ ] Safety proofs for memory-critical code paths
- [ ] Bounded model checking for finite-state properties
- [ ] `agamc verify` command for running verification

## Responsible Crates

- `agam_smt` — SMT integration, verification engine
- `agam_sema` — contract checking, annotation processing

## Dependencies

- Tiers 0–2 complete — verification needs a stable type system and memory model
