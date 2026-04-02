from __future__ import annotations

if __package__ in {None, ""}:  # pragma: no cover
    import sys
    from pathlib import Path

    sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

import argparse
import json
import os
import subprocess
import tempfile
import time
from pathlib import Path
from typing import Any

from benchmarks.harness.agam_harness import AgamHarness
from benchmarks.harness.base_harness import BaseHarness, PreparedBenchmark
from benchmarks.harness.c_harness import CHarness
from benchmarks.harness.cpp_harness import CppHarness
from benchmarks.harness.go_harness import GoHarness
from benchmarks.harness.python_harness import PythonHarness
from benchmarks.harness.rust_harness import RustHarness
from benchmarks.infrastructure.compilation_analyzer import CompilationAnalyzer
from benchmarks.infrastructure.cpu_profiler import CpuProfiler, CpuSample
from benchmarks.infrastructure.memory_profiler import MemoryProfiler
from benchmarks.infrastructure.result_formatter import ResultFormatter
from benchmarks.infrastructure.statistical_analyzer import StatisticalAnalyzer
from benchmarks.infrastructure.utils import (
    CONFIG_ROOT,
    RESULT_ROOT,
    REPO_ROOT,
    SUITE_ROOT,
    benchmark_name_for,
    complexity_hint_for,
    current_environment_name,
    discover_benchmarks,
    ensure_directory,
    host_metadata,
    load_yaml_like,
    parse_csv_arguments,
    sanitize_preview,
    sha256_text,
    timestamp_label,
)


class BenchmarkWorkspace:
    def __init__(self, environment_name: str | None = None) -> None:
        self.config = load_yaml_like(CONFIG_ROOT / "benchmark_config.yaml")
        self.environments = load_yaml_like(CONFIG_ROOT / "environments.yaml")
        self.targets = load_yaml_like(CONFIG_ROOT / "comparison_targets.yaml")
        self.environment_name = environment_name or current_environment_name()
        self.environment = self.environments[self.environment_name]
        self.platform_name = str(self.environment.get("platform_name", self.environment_name))
        self.harnesses: list[BaseHarness] = [
            AgamHarness(self.environment, self.targets),
            RustHarness(self.environment, self.targets),
            PythonHarness(self.environment, self.targets),
            CHarness(self.environment, self.targets),
            CppHarness(self.environment, self.targets),
            GoHarness(self.environment, self.targets),
        ]
        self.cpu_profiler = CpuProfiler()
        self.cpu_topology = self.cpu_profiler.detect_topology()
        self.memory_profiler = MemoryProfiler()
        self.compilation_analyzer = CompilationAnalyzer()
        self.statistics = StatisticalAnalyzer()
        self.formatter = ResultFormatter()

    def harness_for(self, source: Path) -> BaseHarness:
        for harness in self.harnesses:
            if harness.can_handle(source):
                return harness
        raise ValueError(f"No harness registered for {source}")

    def run(
        self,
        suites: list[str] | None,
        include_comparisons: bool,
        language_filters: set[str] | None,
        target_filters: list[str] | None,
        warmups: int | None,
        runs: int | None,
        max_benchmarks: int | None,
        dry_run: bool,
    ) -> dict[str, Any]:
        defaults = self.config["defaults"]
        warmup_runs = warmups if warmups is not None else defaults["warmup_runs"]
        measured_runs = runs if runs is not None else defaults["measured_runs"]
        selected_targets = target_filters or defaults["default_targets"]
        target_filter_set = set(selected_targets)
        benchmarks = discover_benchmarks(
            suite_filters=suites,
            include_comparisons=include_comparisons or defaults["include_comparisons"],
            language_filters=language_filters,
        )
        if max_benchmarks is not None:
            benchmarks = benchmarks[:max_benchmarks]

        run_label = timestamp_label()
        run_root = ensure_directory(RESULT_ROOT / "raw" / run_label)
        aggregated_root = ensure_directory(RESULT_ROOT / "aggregated")
        reports_root = ensure_directory(RESULT_ROOT / "reports")
        ensure_directory(RESULT_ROOT / "plots")
        build_root = ensure_directory(run_root / "build")

        performance_rows: list[dict[str, Any]] = []
        memory_rows: list[dict[str, Any]] = []
        compilation_rows: list[dict[str, Any]] = []

        for source in benchmarks:
            harness = self.harness_for(source)
            case_name = benchmark_name_for(source)
            suite = source.relative_to(SUITE_ROOT).parts[0]
            complexity = complexity_hint_for(source)
            prepared_variants = harness.prepare_variants(
                source,
                build_root / case_name,
                target_filters=target_filter_set,
            )

            for prepared in prepared_variants:
                row_context = self._row_context(source, suite, case_name, prepared, complexity)
                compile_row = self.compilation_analyzer.measure(
                    prepared.compile_command,
                    cwd=REPO_ROOT,
                    env=os.environ.copy(),
                    artifact_path=prepared.artifact_path,
                )
                if compile_row is not None:
                    compile_row.update(row_context)
                    compilation_rows.append(compile_row)
                    if compile_row["return_code"] != 0:
                        memory_rows.append(
                            {
                                **row_context,
                                **self.memory_profiler.build_space_profile(
                                    source,
                                    prepared.artifact_path,
                                    prepared.runtime_executable,
                                    self.cpu_topology,
                                    peak_rss_bytes=None,
                                ),
                            }
                        )
                        continue

                if dry_run:
                    memory_rows.append(
                        {
                            **row_context,
                            **self.memory_profiler.build_space_profile(
                                source,
                                prepared.artifact_path,
                                prepared.runtime_executable,
                                self.cpu_topology,
                                peak_rss_bytes=None,
                            ),
                        }
                    )
                    continue

                for _ in range(warmup_runs):
                    self._execute(prepared.run_command)

                samples_ms: list[float] = []
                peak_rss_samples: list[int] = []
                stdout_hashes: list[str] = []
                return_codes: list[int] = []
                for _ in range(measured_runs):
                    result = self._execute(prepared.run_command)
                    samples_ms.append(result["wall_time_ms"])
                    if result["peak_rss_bytes"] is not None:
                        peak_rss_samples.append(result["peak_rss_bytes"])
                    stdout_hashes.append(result["stdout_sha256"])
                    return_codes.append(result["return_code"])

                summary = self.statistics.summarize(samples_ms)
                performance_rows.append(
                    {
                        **row_context,
                        "median_ms": summary["median_ms"],
                        "mean_ms": summary["mean_ms"],
                        "delta_percent": summary.get("delta_percent"),
                        "sample_count": summary["sample_count"],
                        "stdout_hash": stdout_hashes[-1] if stdout_hashes else None,
                        "return_codes": return_codes,
                        "summary": summary,
                    }
                )
                memory_rows.append(
                    {
                        **row_context,
                        **self.memory_profiler.build_space_profile(
                            source,
                            prepared.artifact_path,
                            prepared.runtime_executable,
                            self.cpu_topology,
                            peak_rss_bytes=max(peak_rss_samples) if peak_rss_samples else None,
                        ),
                    }
                )

        metadata = {
            "environment": self.environment_name,
            "platform_name": self.platform_name,
            "host": host_metadata(),
            "cpu_topology": self.cpu_topology.to_dict(),
            "benchmark_count": len(benchmarks),
            "warmup_runs": warmup_runs,
            "measured_runs": measured_runs,
            "dry_run": dry_run,
            "selected_suites": suites or [],
            "selected_targets": selected_targets,
            "selected_languages": sorted(language_filters) if language_filters else [],
        }
        self.formatter.write(
            run_root=run_root,
            performance=performance_rows,
            memory=memory_rows,
            compilation=compilation_rows,
            metadata=metadata,
            aggregated_root=aggregated_root,
            reports_root=reports_root,
        )
        return {
            "run_root": str(run_root),
            "performance_rows": len(performance_rows),
            "memory_rows": len(memory_rows),
            "compilation_rows": len(compilation_rows),
            "metadata": metadata,
        }

    def _row_context(
        self,
        source: Path,
        suite: str,
        case_name: str,
        prepared: PreparedBenchmark,
        complexity: dict[str, str | None],
    ) -> dict[str, Any]:
        return {
            "platform": self.platform_name,
            "suite": suite,
            "case": case_name,
            "language": prepared.language,
            "target_id": prepared.target_id,
            "target_name": prepared.target_name,
            "backend": prepared.backend,
            "compiler": prepared.compiler,
            "call_cache_enabled": prepared.call_cache_enabled,
            "source": str(source.relative_to(REPO_ROOT)),
            "time_complexity": complexity.get("time_complexity"),
            "space_complexity": complexity.get("space_complexity"),
            "complexity_notes": complexity.get("complexity_notes"),
        }

    def _execute(self, command: list[str]) -> dict[str, Any]:
        start = time.perf_counter_ns()
        with tempfile.TemporaryFile() as stdout_file, tempfile.TemporaryFile() as stderr_file:
            process = subprocess.Popen(
                command,
                cwd=REPO_ROOT,
                stdout=stdout_file,
                stderr=stderr_file,
            )
            peak_rss_bytes = self.memory_profiler.capture_peak_rss_bytes(process)
            return_code = process.wait()
            wall_time_ms = (time.perf_counter_ns() - start) / 1_000_000
            sample = CpuSample(wall_time_ms=round(wall_time_ms, 6), return_code=return_code)

            stdout_file.seek(0)
            stderr_file.seek(0)
            stdout_text = stdout_file.read().decode("utf-8", errors="replace")
            stderr_text = stderr_file.read().decode("utf-8", errors="replace")

        return {
            "wall_time_ms": sample.wall_time_ms,
            "return_code": sample.return_code,
            "peak_rss_bytes": peak_rss_bytes,
            "stdout_sha256": sha256_text(stdout_text),
            "stdout_preview": sanitize_preview(stdout_text),
            "stderr_preview": sanitize_preview(stderr_text),
        }


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Run Agam benchmark suites.")
    parser.add_argument("--suite", action="append", dest="suites", help="Suite directory name")
    parser.add_argument(
        "--language",
        action="append",
        dest="languages",
        choices=["agam", "rust", "python", "c", "cpp", "go"],
        help="Restrict runs to one or more languages",
    )
    parser.add_argument(
        "--target",
        action="append",
        dest="targets",
        help="Target ids from comparison_targets.yaml",
    )
    parser.add_argument(
        "--environment",
        help="Environment id from environments.yaml",
    )
    parser.add_argument(
        "--include-comparisons",
        action="store_true",
        help="Include sources under comparisons/ directories",
    )
    parser.add_argument("--warmups", type=int, help="Override warmup count")
    parser.add_argument("--runs", type=int, help="Override measured run count")
    parser.add_argument("--max-benchmarks", type=int, help="Limit discovered benchmarks")
    parser.add_argument("--dry-run", action="store_true", help="Discover and compile only")
    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    workspace = BenchmarkWorkspace(environment_name=args.environment)
    result = workspace.run(
        suites=parse_csv_arguments(args.suites),
        include_comparisons=args.include_comparisons,
        language_filters=set(args.languages) if args.languages else None,
        target_filters=parse_csv_arguments(args.targets),
        warmups=args.warmups,
        runs=args.runs,
        max_benchmarks=args.max_benchmarks,
        dry_run=args.dry_run,
    )
    print(json.dumps(result, indent=2))
    return 0


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
