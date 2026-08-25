# Phase T6-ai-compiler-intel � AI-Native Compiler Intelligence

**Status:** open
**Tier:** 6 (Frontier and Long-Horizon)

## Vision

Move the compiler from a rigid rule engine to an **intelligent pipeline** that uses machine learning to optimize code, generate human-readable error explanations, and auto-tune performance.

## Deliverables

### Semantic Error Messages (near-term, aligns with DX1)
- [ ] Context-aware error analysis: "You declared `x` as `i32` on line 5 but passed it to a function expecting `String` on line 12"
- [ ] "Did you mean?" suggestions using edit-distance AND type-compatibility analysis
- [ ] Common mistake patterns database with natural-language explanations
- [ ] Link to relevant documentation for each error code

### Profile-Guided AI Optimization
- [ ] Lightweight ML model trained on PGO profiles to predict hot paths
- [ ] Auto-tuning of inlining thresholds based on historical profile data
- [ ] Predictive branch annotation without requiring manual `@likely`/`@unlikely`
- [ ] Compilation strategy selection: optimize for size vs speed based on workload classification

### Auto-Parallelization Intelligence
- [ ] ML-guided loop parallelization decisions (parallelize when profitable)
- [ ] Automatic GPU offloading decisions based on operation characteristics
- [ ] Thread pool sizing based on hardware and workload profiling

### Code Quality Intelligence
- [ ] Static analysis suggestions: "This loop can be replaced with `map().collect()`"
- [ ] Performance anti-pattern detection: "This allocation inside a hot loop costs 30% of runtime"
- [ ] Security pattern recognition: "This pattern matches CVE-2024-XXXX — consider using X instead"

## Responsible Crates

- `agam_profile` — PGO data collection, ML model training data
- `agam_mir` — ML-guided optimization decisions
- `agam_errors` — semantic error analysis engine
- `agam_lint` — code quality intelligence

## Dependencies

- Phase T1-error-messages (error messages) — shared diagnostic infrastructure
- Phase T4-llvm-optimization (LLVM optimization) — PGO pipeline
- Phase T5-autodiff-tensors (autodiff/tensors) — ML model infrastructure
