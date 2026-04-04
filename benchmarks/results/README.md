# Benchmark Results

This directory is the checked-in result root for the Agam benchmark workspace.

Use [`../README.md`](../README.md) for benchmark inventory and harness usage, and [`../COVERAGE_MATRIX.md`](../COVERAGE_MATRIX.md) for the broader 60-workload implemented-vs-future benchmark map. Use this file to understand which result sets are currently present in git and how to read the files under `raw/`, `aggregated/`, `reports/`, and `plots/`.

## Layout

- `raw/<timestamp>/`
  - one benchmark-harness invocation per directory, including `performance.json`, `compilation.json`, `memory.json`, `metadata.json`, and any emitted build artifacts
- `aggregated/`
  - the latest CSV summaries emitted by the result formatter
- `reports/`
  - the latest Markdown reports emitted by the result formatter
- `plots/`
  - optional chart output when plotting dependencies are available

## Checked-In Result Sets

### Published Same-Host Snapshot

The public same-workload comparison table in [`../../README.md`](../../README.md) is backed by `raw/2026-04-02_17-00-55/`.

- environment: `local_windows_win11`
- host: `Windows-11-10.0.26200-SP0`, AMD64, 8 physical cores, 16 logical cores
- suite: `01_algorithms`
- workload filter: `fibonacci`
- warmups: `2`
- measured runs: `7`
- compile warmup runs: `1`
- selected targets: `agam_llvm_o3_call_cache_off`, `agam_llvm_o3_call_cache_on`, `agam_c_o3_call_cache_off`, `agam_c_o3_call_cache_on`, `agam_jit_o2_call_cache_off`, `agam_jit_o2_call_cache_on`, `rust_release`, `python_cpython`, `c_clang_o3`, `cpp_clangxx_o3`, `go_release`

All rows in that measured snapshot use the same recursive Fibonacci workload with time complexity `O(phi^n)` and space complexity `O(n)`.

| Target | Backend | Runtime median (ms) | Compile time (ms) | SSD footprint | Peak RSS |
| --- | --- | ---: | ---: | ---: | ---: |
| Agam LLVM O3 | LLVM | 22.805 | 183.0 | 150.50 KiB | 3.63 MiB |
| Agam LLVM O3 + Call Cache | LLVM | 12.482 | 173.0 | 150.50 KiB | 3.63 MiB |
| Agam C O3 | C | 23.287 | 505.7 | 163.50 KiB | 3.62 MiB |
| Agam C O3 + Call Cache | C | 23.327 | 450.0 | 163.50 KiB | 3.62 MiB |
| Agam JIT O2 | JIT | 137.818 | n/a | 16.04 MiB | 12.36 MiB |
| Agam JIT O2 + Call Cache | JIT | 147.551 | n/a | 16.04 MiB | 12.36 MiB |
| Clang++ O3 | native | 22.753 | 801.0 | 257.00 KiB | 3.69 MiB |
| Clang C O3 | native | 23.479 | 118.3 | 135.00 KiB | 3.61 MiB |
| Rust release | native | 23.800 | 190.2 | 126.00 KiB | 4.02 MiB |
| Go release | native | 33.822 | 192.9 | 2.35 MiB | 5.61 MiB |
| CPython | interpreted | 359.203 | n/a | 101.96 KiB | 11.66 MiB |

Measured raw files for that snapshot live under:

- `raw/2026-04-02_17-00-55/performance.json`
- `raw/2026-04-02_17-00-55/compilation.json`
- `raw/2026-04-02_17-00-55/memory.json`
- `raw/2026-04-02_17-00-55/metadata.json`

### New-Workload Dry-Run Validation

`raw/2026-04-04_07-25-36/` is the newer dry-run validation pass for the 9 workloads added to broaden the suite beyond Fibonacci.

- environment: `local_windows_win11`
- host: `Windows-11-10.0.26200-SP0`, AMD64, 8 physical cores, 16 logical cores
- `dry_run`: `true`
- selected target: `agam_llvm_o3_call_cache_off`
- benchmark count: `9`
- warmups: `2`
- measured runs: `7`
- compile warmup runs: `1`

Validated workloads:

- `01_algorithms/edit_distance.agam`
- `02_numerical_computation/polynomial_eval.agam`
- `03_data_structures/ring_buffer.agam`
- `06_string_processing/token_frequency.agam`
- `07_io_operations/csv_scanning.agam`
- `08_jit_optimization/call_cache_hotset.agam`
- `08_jit_optimization/call_cache_mixed_locality.agam`
- `08_jit_optimization/call_cache_phase_shift.agam`
- `08_jit_optimization/call_cache_unique_inputs.agam`

Because that invocation used `dry_run`, it does not contain measured runtime rows and its compilation durations are intentionally `null`. The useful signal in this run is harness coverage: the workspace discovered the new workloads, planned them correctly, and populated the expected build-root entries under `raw/2026-04-04_07-25-36/build/`.

## Why The Aggregated Files Look Partial

The formatter outputs under `aggregated/` and `reports/` always reflect the latest run written to `results/`.

In the current checked-in tree, the latest run is the `2026-04-04_07-25-36` dry-run validation. That is why:

- `aggregated/performance_summary.csv` is header-only
- `aggregated/scorecard_summary.csv` is header-only
- `reports/PERFORMANCE_REPORT.md` reports no rows
- `reports/COMPILATION_REPORT.md` and `reports/MEMORY_REPORT.md` list only the 9 new dry-run workloads

This is expected. It does not mean the April 2 measured snapshot disappeared; it only means the latest formatter output was produced from the newer dry-run validation rather than from a full measured matrix.

## Rebuilding A Full Nearby Result Set

To refresh `aggregated/`, `reports/`, and `plots/` with measured runtime data instead of the current dry-run view, rerun the harness without `--dry-run`. For example:

```bash
python -m benchmarks.infrastructure.benchmark_harness \
  --environment local_windows_win11 \
  --suite 01_algorithms \
  --match fibonacci \
  --include-comparisons \
  --target agam_llvm_o3_call_cache_off \
  --target agam_llvm_o3_call_cache_on \
  --target agam_c_o3_call_cache_off \
  --target agam_c_o3_call_cache_on \
  --target agam_jit_o2_call_cache_off \
  --target agam_jit_o2_call_cache_on \
  --target rust_release \
  --target python_cpython \
  --target c_clang_o3 \
  --target cpp_clangxx_o3 \
  --target go_release \
  --warmups 2 \
  --runs 7
```

For call-cache evaluation, do not treat one benchmark as the whole story. The benchmark workspace now includes dedicated hot-set, mixed-locality, phase-shift, and unique-input call-cache cases under `../benchmarks/08_jit_optimization/` so backend decisions can be based on locality shape instead of Fibonacci alone.
