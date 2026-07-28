# Phase T1-project-completion-estimate — Planning Note: Project Completion Snapshot

**Status:** planning note (not a roadmap build phase)

**Date:** 2026-05-29

## Assumptions

- This estimate is for the current roadmap in `.agent/specs/active/catalog.md`.
- Status scoring model:
  - `complete = 1.00`
  - `partial = 0.50`
  - `decided = 0.25`
  - `open = 0.00`
- "Full project" means the whole current T0-T6 roadmap, not only the near-term compiler product path.
- Team-speed assumptions:
  - small team = 2-5 strong full-time compiler engineers
  - solo / part-time work will take materially longer

## Headline Answer

If the **entire current roadmap = 100%**, Agam is **about 21% complete**.

That number is strict and roadmap-wide. It includes long-horizon T4-T6 work such as advanced optimization depth, AI-native features, formal verification, and self-hosting.

If the denominator is instead **"shipping a serious compiler/toolchain v1 on the current native LLVM product path"**, the project is **roughly 55-60% complete**.

## How 21% Was Computed

Tier inventory from `catalog.md`:

| Tier | Complete | Partial | Decided | Open | Total |
| --- | ---: | ---: | ---: | ---: | ---: |
| T0 | 3 | 1 | 0 | 4 | 8 |
| T1 | 8 | 4 | 0 | 10 | 22 |
| T2 | 1 | 0 | 1 | 8 | 10 |
| T3 | 1 | 1 | 0 | 13 | 15 |
| T4 | 0 | 0 | 0 | 11 | 11 |
| T5 | 0 | 0 | 0 | 7 | 7 |
| T6 | 0 | 0 | 0 | 4 | 4 |
| **Total** | **13** | **6** | **1** | **57** | **77** |

Weighted completion score:

- `13 * 1.00 = 13.00`
- `6 * 0.50 = 3.00`
- `1 * 0.25 = 0.25`
- total score = `16.25`

Completion percent:

- `16.25 / 77 = 21.1%`

Rounded planning answer:

- **Strict roadmap-wide completion:** **~21%**

## Product-Path Completion Estimate

If the denominator is narrowed to **"usable production-path compiler/toolchain"**, completion is much higher because many enabling systems are already shipped:

- lexer, parser, sema, HIR, MIR, first backends
- daemon/prewarm
- CLI/product shell
- resolver/lockfile/workspace model
- REPL/headless exec
- OS sandbox baseline
- target profiles
- partial GPU/NPU path

But core blockers remain:

- T0-type-system
- T0-object-model
- T0-module-system
- T1-error-messages
- T1-lsp-production
- remaining SDK/headless exec hardening
- MIR lowering gaps for aggregate language features

**Product-path estimate:** **~55-60% complete**

## Time Estimate by Milestone

### 1. Strong Daily-Use Compiler

Goal:

- native LLVM path stable
- core language less brittle
- better diagnostics
- workable package/build/dev loop

Estimate:

- **2 strong full-time engineers:** `6-12 months`
- **solo / part-time:** `18-36 months`

### 2. Production v1 Toolchain

Goal:

- T0 foundation mostly closed
- T1 tooling professionalized
- T2 runtime/security story credible
- release/SDK/distribution path repeatable

Estimate:

- **3-5 strong engineers:** `12-24 months`
- **solo / part-time:** `2-4 years`

### 3. Whole Roadmap

Goal:

- T0-T6 substantially complete, including optimization depth, AI-native tiers, frontier work

Estimate:

- **small focused team:** `3-5+ years`
- **solo / part-time:** `many years`

## Time Estimate by Tier

| Tier | Remaining Work | Rough Time |
| --- | --- | --- |
| T0 | hardest language foundation still open | `6-10 months` |
| T1 | tooling, diagnostics, LSP, docs, testing | `4-8 months` parallel with T0 |
| T2 | memory/security/runtime depth | `6-12 months` |
| T3 | platform/ecosystem depth | `8-14 months` |
| T4 | optimization depth | `12-24 months` |
| T5 | AI-native differentiation | `12-24 months` |
| T6 | frontier / research | `12-36+ months` |

These are overlapping, not strictly sequential.

## Highest-Leverage Remaining Phases

If schedule matters, these phases dominate time-to-v1:

1. `T0-type-system`
2. `T0-object-model`
3. `T0-module-system`
4. `T1-error-messages`
5. `T1-lsp-production`
6. `T1-sdk-distribution`
7. `T1-headless-exec`
8. `T3-gpu-npu-pipeline`

## Detailed A-Z Roadmap

This is the practical end-to-end roadmap from current state to long-horizon vision. It is ordered by dependency and leverage, not by marketing priority.

### A. Aggregate MIR gaps

Focus:

- close current MIR lowering gaps for structs, enums, and `match`
- remove obvious "half-wired language surface" failures before broader feature growth

Maps to:

- current MIR todo gaps
- `T0-object-model`
- `T0-type-system`

Exit signal:

- aggregate language features lower end-to-end through MIR and at least one production backend

### B. Base stdlib maturity

Focus:

- finish practical stdlib essentials around I/O, paths, errors, and deterministic utility surface
- make stdlib good enough to support real examples and package growth

Maps to:

- `T0-stdlib-io`
- `T0-stdlib-completion`

Exit signal:

- real examples stop depending on compiler-only demos and begin depending on believable stdlib APIs

### C. Core type system

Focus:

- generics
- sum types
- pattern matching
- inference
- `Option` / `Result`

Maps to:

- `T0-type-system`

Exit signal:

- collections, safe error handling, and algebraic data modeling feel native instead of missing

### D. Diagnostics and parser recovery

Focus:

- multi-span errors
- suggestions
- recovery
- stable machine-readable diagnostics

Maps to:

- `T1-error-messages`

Exit signal:

- compiler errors become actionable for normal users, not only compiler contributors

### E. Effects depth and language ergonomics

Focus:

- closures
- named args
- destructuring
- ranges
- operator overloading where justified

Maps to:

- `T0-effects-depth`

Exit signal:

- everyday language ergonomics improve without violating core language coherence

### F. Full module system

Focus:

- file-to-module mapping
- imports
- re-exports
- visibility

Maps to:

- `T0-module-system`

Exit signal:

- multi-file packages become first-class and predictable

### G. Great object model

Focus:

- struct / trait / impl semantics
- constructors
- `self`
- method dispatch
- visibility rules

Maps to:

- `T0-object-model`

Exit signal:

- object-oriented and API-style code lowers coherently through full compiler pipeline

### H. Harden headless execution

Focus:

- stronger capability limits
- filesystem restrictions
- network restrictions
- better isolation contract

Maps to:

- `T1-headless-exec`
- `T2-sandboxing`

Exit signal:

- `agamc exec` is credible for agent and automation usage under hostile input

### I. IDE and LSP production quality

Focus:

- go-to-definition
- completion
- hover
- refactoring support

Maps to:

- `T1-lsp-production`

Exit signal:

- editor experience becomes good enough for daily language development

### J. Judged release path and SDK distribution

Focus:

- release validation on hosted runners
- end-to-end SDK publishing
- artifact verification

Maps to:

- `T1-sdk-distribution`

Exit signal:

- fresh users can install and build without bespoke maintainer setup

### K. Kernel and GPU depth

Focus:

- richer GPU memory model
- kernel ABI maturity
- occupancy-minded launch model
- shared memory depth

Maps to:

- `T3-gpu-npu-pipeline`
- `T4-gpu-optimization-depth`

Exit signal:

- GPU path handles serious kernels, not only smoke-test examples

### L. Linting, formatting, docs

Focus:

- formatter stability
- lint rules
- `agamc doc`
- style coherence

Maps to:

- `T1-formatter-linter`
- `T1-doc-generation`

Exit signal:

- language style becomes teachable, enforceable, and publishable

### M. Memory model implementation

Focus:

- turn decided ownership model into real runtime/compiler behavior
- align ARC/value semantics with optimizer and user model

Maps to:

- `T2-memory-model`

Exit signal:

- ownership and lifetime behavior stop being design-only and become executable semantics

### N. Networking and async runtime

Focus:

- async/await
- channels
- event loop
- TCP/UDP/HTTP/TLS foundations

Maps to:

- `T2-async-concurrency`
- `T2-networking-stack`

Exit signal:

- modern service/backend workloads become viable in native Agam

### O. Observability and tracing

Focus:

- traces
- metrics
- OTLP/OpenTelemetry

Maps to:

- `T2-observability`

Exit signal:

- production debugging and performance measurement stop depending on ad hoc hooks

### P. Package and build maturity

Focus:

- `add/remove/update/bench`
- cross-platform packaging
- better build graph ergonomics

Maps to:

- `T1-build-system`
- `T1-cross-platform-pkg`

Exit signal:

- package lifecycle feels like a real ecosystem, not a compiler demo shell

### Q. Quality engineering

Focus:

- mature test runner
- property tests
- fuzzing
- doctests
- coverage

Maps to:

- `T1-testing-framework`
- `T1-advanced-testing`

Exit signal:

- regression control improves enough to accelerate, not slow, feature delivery

### R. Registry, supply chain, governance

Focus:

- signing
- SBOM/provenance
- plugin/package trust boundaries
- long-term registry model

Maps to:

- `T1-supply-chain-sec`
- `T1-plugin-system`
- `T1-decentralized-registry`

Exit signal:

- package ecosystem becomes trustworthy enough for serious adoption

### S. Sandboxing and security model

Focus:

- language-level permissions
- stronger runtime guardrails
- data/control safety layers

Maps to:

- `T2-sandboxing`
- `T2-cybersecurity`

Exit signal:

- security story is coherent across language, runtime, and tooling

### T. Target expansion

Focus:

- SPIR-V
- WASM
- cross-compilation
- RISC-V

Maps to:

- `T3-spirv-backend`
- `T3-wasm-backend`
- `T3-cross-compilation`
- `T3-riscv-backend`

Exit signal:

- Agam stops being mostly one backend story and becomes a multi-target platform

### U. Universal FFI and external integration

Focus:

- C/C++ interop
- Python/Rust/JS/JVM bridges
- low-overhead hot paths

Maps to:

- `T3-universal-ffi`
- `T1-python-wrappers`

Exit signal:

- external adoption becomes easier before full ecosystem maturity

### V. Visual and native app stack

Focus:

- renderer
- state model
- declarative UI
- design system

Maps to:

- `T3-native-renderer`
- `T3-gpu-rendering`
- `T3-reactive-state`
- `T3-declarative-ui`
- `T3-design-system`

Exit signal:

- desktop/mobile UI becomes a first-party language story, not only a library story

### W. Web and mobile deployment maturity

Focus:

- browser embedding
- WASI/component model
- Android/iOS target hardening

Maps to:

- `T3-wasm-backend`
- `T3-mobile-targets`

Exit signal:

- Agam applications deploy credibly across browser, server, and mobile surfaces

### X. eXtreme optimization depth

Focus:

- LLVM 22.1 optimization depth
- PGO/LTO
- incremental compile
- parallel build
- hardware introspection
- multi-level IR

Maps to:

- `T4-llvm-optimization`
- `T4-comptime-execution`
- `T4-incremental-compile`
- `T4-parallel-build`
- `T4-link-time-opts`
- `T4-hardware-introspection`
- `T4-multi-level-ir`
- `T4-metaprogramming`

Exit signal:

- compiler becomes performance-competitive on serious workloads, not only feature-complete

### Y. Yield AI-native differentiation

Focus:

- autodiff
- tensors
- neural DSL
- distributed training
- edge inference
- probabilistic programming

Maps to:

- `T5-autodiff-tensors`
- `T5-llm-inference`
- `T5-neural-net-dsl`
- `T5-distributed-training`
- `T5-edge-ai-inference`
- `T5-probabilistic-prog`
- `T5-tensor-core-matrix`

Exit signal:

- Agam becomes meaningfully different from general-purpose languages in AI/ML workflows

### Z. Zenith milestones

Focus:

- self-hosting
- formal verification
- AI-guided compiler intelligence
- research features

Maps to:

- `T6-self-hosting`
- `T6-formal-verification`
- `T6-ai-compiler-intel`
- `T6-language-research`

Exit signal:

- project crosses from ambitious compiler product into frontier language platform

## Practical Conclusion

- **Whole roadmap denominator:** about **21% complete**
- **Production-path compiler/toolchain denominator:** about **55-60% complete**
- **Fastest route to real product value:** finish T0 foundation before chasing broad Tier 4-6 ambition
