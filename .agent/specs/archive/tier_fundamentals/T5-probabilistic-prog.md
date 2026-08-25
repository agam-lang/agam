# Phase T5-probabilistic-prog � Probabilistic Programming Primitives

**Status:** open
**Tier:** 5 (AI-Native Differentiation)
**Pillar:** 3 (Math & AI)

## Vision

Add language-native probabilistic programming constructs, enabling Bayesian inference, uncertainty quantification, and probabilistic ML models as first-class Agam programs — not library wrappers.

## Motivation

In 2026, probabilistic programming is converging with differentiable programming:
- **GenJAX** (POPL 2026): Vectorized programmable inference with JAX-like performance
- **MultiPPL** (OOPSLA 2025): Cross-language probabilistic interoperability
- **NumPyro/PyMC**: GPU-accelerated Bayesian inference via JIT compilation

Agam's unique combination of native autodiff (Phase T5-autodiff-tensors), GPU compilation (Phase T4-gpu-optimization-depth), and effects system (Phase 20) provides an ideal foundation for probabilistic programming — effects can model sampling and conditioning as algebraic effects.

## Deliverables

### Probabilistic Types and Syntax
- [ ] `Distribution<T>` type family: `Normal`, `Bernoulli`, `Categorical`, `Dirichlet`, etc.
- [ ] `sample` keyword: draw from a distribution (modeled as an algebraic effect)
- [ ] `observe` keyword: condition on observed data (likelihood)
- [ ] `@model` annotation on functions: marks probabilistic model entry point
- [ ] Type-safe shape checking for distribution parameters (reuses Tensor<T, Shape>)

### Inference Engines
- [ ] Hamiltonian Monte Carlo (HMC) inference backend
- [ ] Variational Inference (VI) via autodiff (reuses Phase T5-autodiff-tensors gradient infrastructure)
- [ ] Importance sampling with particle filtering
- [ ] GPU-accelerated parallel chain execution (reuses Phase T4-gpu-optimization-depth GPU pipeline)
- [ ] `infer(model, data, method: :hmc)` top-level inference API

### Compiler Transforms
- [ ] `sample`/`observe` lowered as algebraic effects (reuses Phase 20 effect system)
- [ ] Automatic log-probability accumulation during model execution
- [ ] Efficient gradient computation through probabilistic programs (Enzyme-style AD)
- [ ] Vectorized model traces for parallel inference (`vmap` over model execution)

## Design References

- **GenJAX (POPL 2026)**: Vectorized programmable inference. Composable `vmap` over model traces.
- **Stan**: Gold-standard HMC inference. Compiles probabilistic models to C++.
- **NumPyro**: JAX-based Bayesian inference. JIT compilation + GPU acceleration.
- **Agam advantage**: Effects system provides natural semantics for `sample`/`observe`. Native autodiff eliminates the need for external gradient computation.

## Responsible Crates

- `agam_sema` — probabilistic type checking, `@model` validation
- `agam_mir` — `sample`/`observe` → effect lowering
- `agam_std` — distribution library, inference engines
- `agam_codegen` — GPU-accelerated inference codegen

## Dependencies

- Phase T5-autodiff-tensors (autodiff) — gradient computation for HMC and VI
- Phase T0-type-system (type system) — generic distribution types
- Phase 20 (effects) — `sample`/`observe` as algebraic effects (already complete)
- Phase T4-gpu-optimization-depth (GPU) — GPU-accelerated parallel inference chains
