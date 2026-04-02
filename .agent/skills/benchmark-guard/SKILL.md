---
name: benchmark-guard
description: Use when changes claim performance wins, affect optimization passes, call caching, specialization, runtime hot paths, or need benchmark-driven validation before acceptance.
---

# Benchmark Guard

Use this skill when the task changes performance-sensitive code or claims speed improvements.

## Workflow

1. Identify the hot path actually affected.
2. Read `references/benchmark-sources.md`.
3. Establish a baseline or reuse a trustworthy recent baseline.
4. Measure the narrowest relevant before/after comparison.
5. Reject changes that regress compile time materially or slow runtime on the target path.
6. Record the measured result in the response or docs when the change is phase-closing.

## Focus Areas

- `agam_profile`
- MIR optimizations
- LLVM proof and attribute changes
- runtime caching and specialization
- benchmark suites under `benchmarks/`
- localized benchmark artifacts under `.agent/test/`

## Success Criteria

- The benchmark matches the changed hot path
- Before/after evidence is explicit
- Regressions are surfaced instead of rationalized
