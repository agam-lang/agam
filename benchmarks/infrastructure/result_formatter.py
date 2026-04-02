from __future__ import annotations

import csv
import json
from pathlib import Path
from typing import Any

from benchmarks.infrastructure.utils import ensure_directory


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
