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
2. Warm up ahead-of-time compile paths separately before recording compile metrics.
3. Collect multiple measured samples per case.
4. Use medians for central tendency and coefficient of variation for noise detection.
5. Compare against a declared baseline rather than against memory.
6. Separate Agam-native runs from comparison-language runs in reporting, then show side-by-side deltas.
7. Compare like with like: platform, backend, compiler/runtime target, and call-cache state are all part of the experiment key.

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

- Default compile warmup runs: `1`
- Rationale: absorb first-hit driver/toolchain startup noise before the measured AOT compile sample
- Scope: applies to AOT targets such as LLVM, C, Rust, Clang, Clang++, Go, and similar native build steps
- Exclusion: JIT and interpreted targets do not invent a fake AOT compile number

## Memory Measurements

When the host platform supports live RSS sampling, capture peak resident set size during execution.
On hosts without a supported sampler, record that memory capture was unavailable instead of inventing a number.

The benchmark workspace also records a space profile for each case:

- SSD footprint
  - produced binary size when a standalone artifact exists
  - source/runtime entrypoint size for interpreted targets
- RAM footprint
  - peak RSS / working-set size during execution
- cache and register context
  - host L1/L2/L3 cache capacity
  - cache-line size
  - pointer width
  - SIMD register width
  - estimated register-file size budget by architecture

Important limitation:

- L3 cache occupancy and live register allocation are not measured precisely in a portable way by this workspace today.
- The current report exposes host cache capacity plus register-budget estimates so cross-platform comparisons remain explicit instead of silently omitted.
- If precise cache-miss or register-pressure counters are needed, add platform-specific perf tooling as a follow-up slice.

## Complexity Annotations

Each benchmark row carries declared algorithmic time and space complexity.

- These complexity tags describe the benchmark workload shape.
- They are not substitutes for measured wall-clock or RSS data.
- If the implementation shape changes, update the complexity hint in the benchmark workspace in the same change.

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
- explicit target subsets across both Linux and Windows runners
- regression detection against a checked-in or downloaded baseline summary

Full benchmark sweeps belong on `workflow_dispatch`, scheduled runs, or explicit release validation.

## Reporting

Each benchmark run should produce:

- raw per-run JSON for traceability
- aggregated CSV and JSON summaries
- human-readable Markdown reports
- metadata that makes reruns reproducible
- explicit platform/backend/target identity in every row

## Reader Scorecard

For same-host, same-workload comparison runs, the workspace can also emit a reader-facing scorecard.

- Use it only when the run holds the workload constant across targets, typically with `--match` or a single benchmark case.
- The current public score uses:
  - `60` points for runtime speed
  - `20` points for peak RAM
  - `10` points for SSD footprint
  - `10` points for ahead-of-time compile latency
- Targets without an ahead-of-time compile row get `0` compile points in that delivery-oriented score.
- The scorecard is a communication layer for public summaries. Raw runtime, compile, and memory rows remain the source of truth.

## Agam-Specific Rules

- Keep `.agam` benchmark sources grounded in syntax already visible in `examples/`, `.agent/test/`, and the active parser.
- Treat `benchmarks/` as the organized benchmark workspace.
- Keep `.agent/test/` for localized phase-work microbenchmarks, legacy generated artifacts, and inspection outputs tied to active optimization slices.
