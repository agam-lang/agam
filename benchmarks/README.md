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
  - global benchmark settings, execution environments, and comparison targets
- `infrastructure/`
  - discovery, execution, profiling, statistics, and result formatting
- `benchmarks/`
  - benchmark suites grouped by workload class
- `harness/`
  - language-specific runners for Agam, Rust, Python, C, and Go
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

Run the default benchmark pass:

```bash
bash benchmarks/run_all_benchmarks.sh
```

Run a narrow suite directly:

```bash
python -m benchmarks.infrastructure.benchmark_harness --suite 01_algorithms --max-benchmarks 3
```

Include comparison-language sources:

```bash
python -m benchmarks.infrastructure.benchmark_harness --suite 01_algorithms --include-comparisons
```

## GitHub Actions And `gh` CLI

The repo workflow lives at `.github/workflows/benchmarks.yml`.

Dispatch a remote benchmark run:

```bash
python -m benchmarks.ci.gh_benchmark_cli run --ref main --suite 08_jit_optimization
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

Aggregated outputs land in:

- `results/aggregated/performance_summary.csv`
- `results/aggregated/memory_summary.csv`
- `results/aggregated/compilation_summary.csv`
- `results/aggregated/statistical_analysis.json`

Reports land in:

- `results/reports/PERFORMANCE_REPORT.md`
- `results/reports/MEMORY_REPORT.md`
- `results/reports/COMPILATION_REPORT.md`
- `results/reports/EXECUTIVE_SUMMARY.md`

Plot generation is optional. If `matplotlib` is available, the formatter can fill `results/plots/`.

