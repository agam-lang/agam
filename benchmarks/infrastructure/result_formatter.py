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
            ("suite", "case", "language", "median_ms", "mean_ms", "delta_percent"),
        )
        self._write_csv(
            aggregated_root / "memory_summary.csv",
            memory,
            ("suite", "case", "language", "peak_rss_bytes"),
        )
        self._write_csv(
            aggregated_root / "compilation_summary.csv",
            compilation,
            ("suite", "case", "language", "duration_ms", "return_code"),
        )
        (aggregated_root / "statistical_analysis.json").write_text(
            json.dumps(performance, indent=2) + "\n",
            encoding="utf-8",
        )

        self._write_report(
            reports_root / "PERFORMANCE_REPORT.md",
            "Performance Report",
            performance,
        )
        self._write_report(
            reports_root / "MEMORY_REPORT.md",
            "Memory Report",
            memory,
        )
        self._write_report(
            reports_root / "COMPILATION_REPORT.md",
            "Compilation Report",
            compilation,
        )
        executive_lines = [
            "# Executive Summary",
            "",
            f"- Recorded cases: {len(performance)}",
            f"- Memory observations: {len(memory)}",
            f"- Compilation observations: {len(compilation)}",
            f"- Environment: {metadata.get('environment')}",
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
    def _write_report(path: Path, title: str, rows: list[dict[str, Any]]) -> None:
        lines = [f"# {title}", ""]
        if not rows:
            lines.append("- No rows recorded.")
        else:
            for row in rows:
                case = row.get("case", "unknown")
                suite = row.get("suite", "unknown")
                language = row.get("language", "unknown")
                lines.append(
                    f"- `{suite}` / `{case}` / `{language}`: {json.dumps(row, sort_keys=True)}"
                )
        path.write_text("\n".join(lines) + "\n", encoding="utf-8")
