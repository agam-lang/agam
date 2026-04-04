# Agam Benchmark Workspace

This directory is the organized benchmark workspace for Agam.

It complements the localized phase-work microbenchmarks under `.agent/test/` with:

- categorized benchmark suites under `benchmarks/benchmarks/`
- reusable harness code under `benchmarks/harness/` and `benchmarks/infrastructure/`
- CI and regression tooling under `benchmarks/ci/`
- generated results under `benchmarks/results/`
- methodology and operating guidance in this folder

## Layout

- `METHODOLOGY.md`
  - scientific measurement rules, baselines, and reporting expectations
- `config/`
  - global benchmark settings, execution environments, and comparison/backend targets
- `infrastructure/`
  - discovery, execution, profiling, statistics, and result formatting
- `benchmarks/`
  - benchmark suites grouped by workload class
- `harness/`
  - language-specific runners for Agam, Rust, CPython, Clang C, Clang++, and Go
- `results/`
  - raw runs, aggregated summaries, reports, and plots
- `ci/`
  - baseline management, regression detection, and CI defaults
- `tests/`
  - focused unit tests for the benchmark workspace

## Local Usage

Prepare optional benchmark dependencies:

```bash
bash benchmarks/setup.sh
```

## Current Inventory

The workspace is broader than the published single-workload snapshot in the top-level README.

- Agam benchmark sources: `38`
- comparison-language sources: `13`
- total runnable benchmark sources in `benchmarks/benchmarks/`: `51`

Current Agam suite counts:

- `01_algorithms`: `5`
- `02_numerical_computation`: `5`
- `03_data_structures`: `4`
- `04_memory_intensive`: `3`
- `05_ml_primitives`: `4`
- `06_string_processing`: `3`
- `07_io_operations`: `3`
- `08_jit_optimization`: `7`
- `09_compilation_metrics`: `4`

Run the default benchmark pass:

```bash
bash benchmarks/run_all_benchmarks.sh
```

Run a narrow suite directly:

```bash
python -m benchmarks.infrastructure.benchmark_harness --suite 01_algorithms --max-benchmarks 3
```

Run one benchmark shape across every backend/runtime target from the same harness invocation:

```bash
python -m benchmarks.infrastructure.benchmark_harness \
  --environment local_windows_win11 \
  --suite 01_algorithms \
  --match fibonacci \
  --include-comparisons
```

The `--match` filter is the easiest way to hold the workload constant across Agam backends and comparison-language targets when you want a same-host, same-situation comparison.

Run explicit Agam backend and call-cache comparisons:

```bash
python -m benchmarks.infrastructure.benchmark_harness \
  --suite 08_jit_optimization \
  --target agam_llvm_o3_call_cache_off \
  --target agam_llvm_o3_call_cache_on \
  --target agam_jit_o2_call_cache_off \
  --target agam_jit_o2_call_cache_on
```

Call-cache study workloads now include several distinct locality shapes instead of only recursive overlap:

- `08_jit_optimization/call_cache_hotset.agam`
- `08_jit_optimization/call_cache_mixed_locality.agam`
- `08_jit_optimization/call_cache_phase_shift.agam`
- `08_jit_optimization/call_cache_unique_inputs.agam`
- `08_jit_optimization/call_cache_profile.agam`
- `08_jit_optimization/specialization_demo.agam`
- `08_jit_optimization/adaptive_optimization.agam`

Run just the call-cache locality set across on/off targets:

```bash
python -m benchmarks.infrastructure.benchmark_harness \
  --suite 08_jit_optimization \
  --match call_cache \
  --target agam_llvm_o3_call_cache_off \
  --target agam_llvm_o3_call_cache_on \
  --target agam_jit_o2_call_cache_off \
  --target agam_jit_o2_call_cache_on
```

Run explicit compiler/runtime comparisons:

```bash
python -m benchmarks.infrastructure.benchmark_harness \
  --suite 01_algorithms \
  --include-comparisons \
  --target python_cpython \
  --target c_clang_o3 \
  --target cpp_clangxx_o3 \
  --target rust_release \
  --target go_release
```

Select an environment profile:

```bash
python -m benchmarks.infrastructure.benchmark_harness \
  --environment local_windows_win11 \
  --suite 01_algorithms \
  --target agam_llvm_o3_call_cache_off
```

Include comparison-language sources:

```bash
python -m benchmarks.infrastructure.benchmark_harness --suite 01_algorithms --include-comparisons
```

## GitHub Actions And `gh` CLI

The repo workflow lives at `.github/workflows/benchmarks.yml`.

Dispatch a remote benchmark run:

```bash
python -m benchmarks.ci.gh_benchmark_cli run --ref main --suite 08_jit_optimization --target agam_jit_o2_call_cache_on
```

List recent benchmark runs:

```bash
python -m benchmarks.ci.gh_benchmark_cli list
```

Download benchmark artifacts:

```bash
python -m benchmarks.ci.gh_benchmark_cli download --run-id 123456789
```

## Result Contract

Each run writes:

- `results/raw/<timestamp>/performance.json`
- `results/raw/<timestamp>/memory.json`
- `results/raw/<timestamp>/compilation.json`
- `results/raw/<timestamp>/metadata.json`

`memory.json` now carries the space-profile view:

- peak RSS in RAM
- on-disk SSD footprint of the produced artifact or source/runtime entrypoint
- host L1/L2/L3 cache capacity metadata
- pointer width, SIMD register width, and register-file budget estimates
- declared time/space complexity tags for each benchmark

Aggregated outputs land in:

- `results/aggregated/performance_summary.csv`
- `results/aggregated/memory_summary.csv`
- `results/aggregated/compilation_summary.csv`
- `results/aggregated/scorecard_summary.csv`
- `results/aggregated/statistical_analysis.json`

Reports land in:

- `results/reports/PERFORMANCE_REPORT.md`
- `results/reports/MEMORY_REPORT.md`
- `results/reports/COMPILATION_REPORT.md`
- `results/reports/EXECUTIVE_SUMMARY.md`

Plot generation is optional. If `matplotlib` is available, the formatter can fill `results/plots/`.

The default config also applies one unmeasured compile warmup (`compile_warmup_runs: 1`) before recording AOT compile-time metrics so one-time driver/toolchain startup costs do not dominate the published numbers.

## Platform And Backend Matrix

The benchmark workspace is designed to compare:

- platforms:
  - local Win11
  - local native Linux
  - WSL Ubuntu 24.04
  - GitHub Actions Linux
  - GitHub Actions Windows
- Agam backends:
  - LLVM `-O3`
  - C backend `-O3`
  - JIT `-O2`
  - call-cache on and off where the CLI supports `--call-cache`
- comparison targets:
  - CPython
  - Clang C `-O3`
  - Clang++ `-O3`
  - Rust release
  - Go release
