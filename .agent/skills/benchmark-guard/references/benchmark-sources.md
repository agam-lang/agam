# Benchmark Sources

Use these sources to ground performance validation.

## Primary Sources

- `agam_profile`
- `benchmarks/benchmarks/01_algorithms/fibonacci.agam`
- `benchmarks/benchmarks/08_jit_optimization/call_cache_profile.agam`
- `benchmarks/benchmarks/09_compilation_metrics/large_program.agam`
- `.agent/test/bench.agam`
- `.agent/test/bench_advanced.agam`
- `.agent/test/bench_call_cache.agam`
- `.agent/test/bench_ml.agam`

## Notes

- Use `benchmarks/` for organized suite coverage and CI-backed regressions.
- Generated artifacts under `.agent/test/` are useful for inspection but should not replace source-based validation.
- Match the benchmark to the subsystem changed rather than picking the largest benchmark by default.
- Use isolated subprocesses for unsafe/JIT-sensitive validation.
