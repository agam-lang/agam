# Benchmark Methodology

This workspace follows a measurement-first process. The benchmark result is part of the engineering claim, not an optional appendix.

## Principles

1. Measure the narrowest benchmark that matches the changed subsystem.
2. Record both runtime and compile-time impact when compiler or backend behavior changed.
3. Keep the environment explicit: toolchain, host, CPU class, OS, and benchmark configuration all belong in the metadata.
4. Prefer repeated runs over single headline numbers.
5. Reject material regressions instead of rationalizing them.

## Experimental Design

1. Warm up the benchmark before recording samples.
2. Collect multiple measured samples per case.
3. Use medians for central tendency and coefficient of variation for noise detection.
4. Compare against a declared baseline rather than against memory.
5. Separate Agam-native runs from comparison-language runs in reporting, then show side-by-side deltas.

## Runtime Measurements

- Default warmup runs: `2`
- Default measured runs: `7`
- Primary statistic: median wall-clock milliseconds
- Secondary statistics: mean, standard deviation, min, max, coefficient of variation

## Compile-Time Measurements

Compile-time metrics should be captured when:

- frontend changes alter parsing or typing cost
- MIR or codegen changes alter backend cost
- LLVM/JIT specialization or cache work changes emitted work per unit

The compile-time contract stores the compile command, wall-clock duration, exit code, and stderr preview for debugging.

## Memory Measurements

When the host platform supports live RSS sampling, capture peak resident set size during execution.
On hosts without a supported sampler, record that memory capture was unavailable instead of inventing a number.

## Regression Thresholds

Use these defaults unless a narrower benchmark contract documents stricter thresholds:

- runtime regression threshold: `5%`
- compile-time regression threshold: `5%`
- coefficient-of-variation warning threshold: `10%`

## Baselines

1. Store baseline summaries from the current default branch or a tagged release.
2. Compare the same suite, target, and execution mode.
3. Refresh baselines intentionally after accepted performance work.
4. Do not overwrite baselines silently.

## CI Strategy

CI should favor a smoke profile by default:

- small suite selection
- limited benchmark count
- stable comparison targets
- regression detection against a checked-in or downloaded baseline summary

Full benchmark sweeps belong on `workflow_dispatch`, scheduled runs, or explicit release validation.

## Reporting

Each benchmark run should produce:

- raw per-run JSON for traceability
- aggregated CSV and JSON summaries
- human-readable Markdown reports
- metadata that makes reruns reproducible

## Agam-Specific Rules

- Keep `.agam` benchmark sources grounded in syntax already visible in `examples/`, `.agent/test/`, and the active parser.
- Treat `benchmarks/` as the organized benchmark workspace.
- Keep `.agent/test/` for localized phase-work microbenchmarks, legacy generated artifacts, and inspection outputs tied to active optimization slices.

