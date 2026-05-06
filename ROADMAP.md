# Roadmap

Agam’s long-term ambition is large, but the near-term roadmap is intentionally product-driven.

## Current Priority Themes

### 1. Native LLVM as the first-class production path

Keep Windows, Linux, and Android on a supportable native LLVM story with real SDK packaging,
toolchain validation, and operational discipline.

### 2. GPU and NPU capability that is compiler-native

Continue Phase 23 by extending the GPU pipeline with richer kernel parameter types, shared memory,
and host/device transfer surfaces without turning the language into a thin wrapper around foreign APIs.

### 3. Effects-aware standard library and execution hardening

Complete the remaining work in:

- effects-aware I/O and networking surfaces
- stronger `agamc exec` capability isolation
- runtime-backed standard library growth that stays aligned with package governance

### 4. Ecosystem integration that uses the same execution contract

Stabilize and publish the external Python integration story while keeping wrappers aligned with the
same `agamc exec --json` contract used elsewhere.

### 5. Developer experience and release discipline

Keep improving:

- contributor onboarding
- CI and release readiness
- benchmark credibility
- docs and runbook accuracy
- repository structure and maintainability

## Not First

These are explicitly not the first priorities:

- broad language-surface sprawl disconnected from the product path
- WSL-only shortcuts that weaken the native host story
- platform expansion that outruns supportability
- performance claims without benchmark evidence

## Where The Detailed Phase Board Lives

The public roadmap is intentionally high level. The repo’s more detailed implementation order lives in:

- [`.agent/phases/next.md`](./.agent/phases/next.md)
- [`.agent/phases/details/`](./.agent/phases/details/)

Those files move faster than this public summary and are the better source for exact work sequencing.
