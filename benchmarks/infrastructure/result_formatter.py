from __future__ import annotations

import csv
import json
from pathlib import Path
from typing import Any

from benchmarks.infrastructure.utils import ensure_directory

try:
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
except ImportError:  # pragma: no cover - optional dependency
    plt = None


class ResultFormatter:
    def write(
        self,
        run_root: Path,
        performance: list[dict[str, Any]],
        memory: list[dict[str, Any]],
        compilation: list[dict[str, Any]],
        metadata: dict[str, Any],
        aggregated_root: Path,
        reports_root: Path,
    ) -> None:
        raw_root = ensure_directory(run_root)
        ensure_directory(aggregated_root)
        ensure_directory(reports_root)

        (raw_root / "performance.json").write_text(
            json.dumps(performance, indent=2) + "\n",
            encoding="utf-8",
        )
        (raw_root / "memory.json").write_text(
            json.dumps(memory, indent=2) + "\n",
            encoding="utf-8",
        )
        (raw_root / "compilation.json").write_text(
            json.dumps(compilation, indent=2) + "\n",
            encoding="utf-8",
        )
        (raw_root / "metadata.json").write_text(
            json.dumps(metadata, indent=2) + "\n",
            encoding="utf-8",
        )

        self._write_csv(
            aggregated_root / "performance_summary.csv",
            performance,
            (
                "platform",
                "suite",
                "case",
                "target_id",
                "target_name",
                "language",
                "backend",
                "compiler",
                "call_cache_enabled",
                "median_ms",
                "mean_ms",
                "delta_percent",
                "time_complexity",
                "space_complexity",
            ),
        )
        self._write_csv(
            aggregated_root / "memory_summary.csv",
            memory,
            (
                "platform",
                "suite",
                "case",
                "target_id",
                "target_name",
                "language",
                "backend",
                "compiler",
                "call_cache_enabled",
                "peak_rss_bytes",
                "ssd_footprint_bytes",
                "artifact_size_bytes",
                "runtime_executable_size_bytes",
                "l1_data_cache_bytes",
                "l2_cache_bytes",
                "l3_cache_bytes",
                "l3_pressure_ratio_estimate",
                "register_file_bytes_estimate",
                "simd_register_width_bytes",
                "pointer_width_bits",
                "time_complexity",
                "space_complexity",
            ),
        )
        self._write_csv(
            aggregated_root / "compilation_summary.csv",
            compilation,
            (
                "platform",
                "suite",
                "case",
                "target_id",
                "target_name",
                "language",
                "backend",
                "compiler",
                "call_cache_enabled",
                "duration_ms",
                "return_code",
                "artifact_size_bytes",
            ),
        )
        (aggregated_root / "statistical_analysis.json").write_text(
            json.dumps(performance, indent=2) + "\n",
            encoding="utf-8",
        )

        self._write_report(
            reports_root / "PERFORMANCE_REPORT.md",
            "Performance Report",
            performance,
            [
                "platform",
                "suite",
                "case",
                "target_name",
                "backend",
                "call_cache_enabled",
                "median_ms",
                "mean_ms",
                "time_complexity",
            ],
        )
        self._write_report(
            reports_root / "MEMORY_REPORT.md",
            "Memory Report",
            memory,
            [
                "platform",
                "suite",
                "case",
                "target_name",
                "peak_rss_bytes",
                "ssd_footprint_bytes",
                "l3_cache_bytes",
                "register_file_bytes_estimate",
                "space_complexity",
            ],
        )
        self._write_report(
            reports_root / "COMPILATION_REPORT.md",
            "Compilation Report",
            compilation,
            [
                "platform",
                "suite",
                "case",
                "target_name",
                "duration_ms",
                "artifact_size_bytes",
            ],
        )
        self._write_plots(aggregated_root.parent / "plots", performance, memory, compilation)
        executive_lines = [
            "# Executive Summary",
            "",
            f"- Recorded performance rows: {len(performance)}",
            f"- Recorded space rows: {len(memory)}",
            f"- Recorded compilation rows: {len(compilation)}",
            f"- Environment: {metadata.get('environment')}",
            f"- Platform: {metadata.get('platform_name')}",
            f"- Selected targets: {', '.join(metadata.get('selected_targets', []))}",
        ]
        (reports_root / "EXECUTIVE_SUMMARY.md").write_text(
            "\n".join(executive_lines) + "\n",
            encoding="utf-8",
        )

    @staticmethod
    def _write_csv(path: Path, rows: list[dict[str, Any]], fieldnames: tuple[str, ...]) -> None:
        with path.open("w", encoding="utf-8", newline="") as handle:
            writer = csv.DictWriter(handle, fieldnames=fieldnames)
            writer.writeheader()
            for row in rows:
                writer.writerow({field: row.get(field) for field in fieldnames})

    @staticmethod
    def _write_report(
        path: Path,
        title: str,
        rows: list[dict[str, Any]],
        keys: list[str],
    ) -> None:
        lines = [f"# {title}", ""]
        if not rows:
            lines.append("- No rows recorded.")
        else:
            for row in rows:
                payload = {key: row.get(key) for key in keys}
                lines.append(f"- {json.dumps(payload, sort_keys=True)}")
        path.write_text("\n".join(lines) + "\n", encoding="utf-8")

    def _write_plots(
        self,
        plots_root: Path,
        performance: list[dict[str, Any]],
        memory: list[dict[str, Any]],
        compilation: list[dict[str, Any]],
    ) -> None:
        if plt is None:
            return

        ensure_directory(plots_root)
        self._plot_bar(
            plots_root / "performance_comparison.png",
            performance,
            value_key="median_ms",
            title="Performance Comparison",
            ylabel="Median runtime (ms)",
        )
        self._plot_bar(
            plots_root / "memory_usage.png",
            memory,
            value_key="peak_rss_bytes",
            title="Memory Usage",
            ylabel="Peak RSS (bytes)",
        )
        self._plot_bar(
            plots_root / "compile_time.png",
            compilation,
            value_key="duration_ms",
            title="Compile Time",
            ylabel="Compile time (ms)",
        )
        self._plot_scatter(
            plots_root / "scaling_analysis.png",
            performance,
            memory,
        )

    def _plot_bar(
        self,
        path: Path,
        rows: list[dict[str, Any]],
        value_key: str,
        title: str,
        ylabel: str,
    ) -> None:
        values = [row for row in rows if isinstance(row.get(value_key), (int, float))]
        if not values or plt is None:
            return

        labels = [f"{row.get('case')}\\n{row.get('target_name')}" for row in values[:12]]
        points = [float(row[value_key]) for row in values[:12]]
        fig, ax = plt.subplots(figsize=(12, 6))
        ax.bar(range(len(points)), points, color="#245c73")
        ax.set_title(title)
        ax.set_ylabel(ylabel)
        ax.set_xticks(range(len(points)))
        ax.set_xticklabels(labels, rotation=45, ha="right")
        fig.tight_layout()
        fig.savefig(path, dpi=144)
        plt.close(fig)

    def _plot_scatter(
        self,
        path: Path,
        performance: list[dict[str, Any]],
        memory: list[dict[str, Any]],
    ) -> None:
        if plt is None:
            return

        memory_index = {
            (row.get("case"), row.get("target_id")): row
            for row in memory
        }
        paired: list[tuple[float, float, str]] = []
        for row in performance:
            runtime = row.get("median_ms")
            memory_row = memory_index.get((row.get("case"), row.get("target_id")))
            footprint = None if memory_row is None else memory_row.get("ssd_footprint_bytes")
            if isinstance(runtime, (int, float)) and isinstance(footprint, (int, float)):
                paired.append((float(runtime), float(footprint), str(row.get("target_name"))))

        if not paired:
            return

        fig, ax = plt.subplots(figsize=(10, 6))
        xs = [item[0] for item in paired]
        ys = [item[1] for item in paired]
        ax.scatter(xs, ys, color="#9f2a2a")
        for x, y, label in paired[:12]:
            ax.annotate(label, (x, y), fontsize=8)
        ax.set_title("Scaling Analysis")
        ax.set_xlabel("Median runtime (ms)")
        ax.set_ylabel("SSD footprint (bytes)")
        fig.tight_layout()
        fig.savefig(path, dpi=144)
        plt.close(fig)
