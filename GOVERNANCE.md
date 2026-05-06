# Governance

Agam is currently maintained as a pre-1.0, maintainer-led language and compiler project.

## Decision Model

Maintainers own final decisions for:

- language and syntax direction
- runtime and ABI contracts
- repository structure and release discipline
- platform support and toolchain policy
- acceptance criteria for large refactors

Community input is welcome, but major changes should not bypass review or project direction.

## How Decisions Are Made

### Small changes

Bug fixes, targeted tooling improvements, doc fixes, and contained refactors are handled through the
normal issue and pull request flow.

### Large changes

Use an issue, design note, or draft PR first for:

- language features
- semantic or runtime model changes
- registry/package contract changes
- SDK or release contract changes
- benchmark methodology changes
- cross-cutting repository restructures

If a large technical change becomes the new project contract, record it as an architecture decision
record under `docs/architecture/decisions/`.

### Performance-sensitive changes

Performance claims need evidence. Benchmark-driven justification is expected for changes that alter:

- code generation
- MIR/HIR optimization behavior
- runtime data structures
- caching or specialization heuristics

Use the performance issue template for measured regressions so the workload, baseline, and host
context stay attached to the report.

## Maintainer Expectations

Maintainers are expected to:

- preserve technical rigor
- keep the public docs and operational runbooks aligned with reality
- capture cross-cutting decisions in durable repo docs instead of leaving them inside PR threads
- avoid hype that outruns the shipped implementation
- prioritize correctness, traceability, and reproducibility

## Path to Broader Governance

As the project grows, governance can evolve toward:

- clearer maintainer roles
- designated area owners
- RFC conventions for language/runtime changes
- more explicit release and support policies

Until then, keep decisions concrete, documented, and grounded in the current repository.
