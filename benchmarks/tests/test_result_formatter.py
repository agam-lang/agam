from __future__ import annotations

import csv
import tempfile
import unittest
from pathlib import Path

from benchmarks.infrastructure.result_formatter import ResultFormatter


class ResultFormatterTests(unittest.TestCase):
    def test_write_emits_scorecard_summary(self) -> None:
        formatter = ResultFormatter()
        performance = [
            {
                "platform": "win11",
                "suite": "01_algorithms",
                "case": "fib_fast",
                "target_id": "fast",
                "target_name": "Fast Target",
                "language": "agam",
                "backend": "llvm",
                "compiler": "agamc",
                "call_cache_enabled": True,
                "median_ms": 10.0,
                "mean_ms": 10.0,
                "delta_percent": None,
                "time_complexity": "O(phi^n)",
                "space_complexity": "O(n)",
            },
            {
                "platform": "win11",
                "suite": "01_algorithms",
                "case": "fib_slow",
                "target_id": "slow",
                "target_name": "Slow Target",
                "language": "c",
                "backend": "native",
                "compiler": "clang",
                "call_cache_enabled": False,
                "median_ms": 20.0,
                "mean_ms": 20.0,
                "delta_percent": None,
                "time_complexity": "O(phi^n)",
                "space_complexity": "O(n)",
            },
        ]
        memory = [
            {
                "platform": "win11",
                "suite": "01_algorithms",
                "case": "fib_fast",
                "target_id": "fast",
                "target_name": "Fast Target",
                "language": "agam",
                "backend": "llvm",
                "compiler": "agamc",
                "call_cache_enabled": True,
                "peak_rss_bytes": 4_000_000,
                "ssd_footprint_bytes": 150_000,
                "artifact_size_bytes": 150_000,
                "runtime_executable_size_bytes": 150_000,
                "l1_data_cache_bytes": 32_768,
                "l2_cache_bytes": 8_388_608,
                "l3_cache_bytes": 16_777_216,
                "l3_pressure_ratio_estimate": 0.2,
                "register_file_bytes_estimate": 640,
                "simd_register_width_bytes": 32,
                "pointer_width_bits": 64,
                "time_complexity": "O(phi^n)",
                "space_complexity": "O(n)",
            },
            {
                "platform": "win11",
                "suite": "01_algorithms",
                "case": "fib_slow",
                "target_id": "slow",
                "target_name": "Slow Target",
                "language": "c",
                "backend": "native",
                "compiler": "clang",
                "call_cache_enabled": False,
                "peak_rss_bytes": 8_000_000,
                "ssd_footprint_bytes": 300_000,
                "artifact_size_bytes": 300_000,
                "runtime_executable_size_bytes": 300_000,
                "l1_data_cache_bytes": 32_768,
                "l2_cache_bytes": 8_388_608,
                "l3_cache_bytes": 16_777_216,
                "l3_pressure_ratio_estimate": 0.4,
                "register_file_bytes_estimate": 640,
                "simd_register_width_bytes": 32,
                "pointer_width_bits": 64,
                "time_complexity": "O(phi^n)",
                "space_complexity": "O(n)",
            },
        ]
        compilation = [
            {
                "platform": "win11",
                "suite": "01_algorithms",
                "case": "fib_fast",
                "target_id": "fast",
                "target_name": "Fast Target",
                "language": "agam",
                "backend": "llvm",
                "compiler": "agamc",
                "call_cache_enabled": True,
                "duration_ms": 100.0,
                "return_code": 0,
                "artifact_size_bytes": 150_000,
            },
            {
                "platform": "win11",
                "suite": "01_algorithms",
                "case": "fib_slow",
                "target_id": "slow",
                "target_name": "Slow Target",
                "language": "c",
                "backend": "native",
                "compiler": "clang",
                "call_cache_enabled": False,
                "duration_ms": 200.0,
                "return_code": 0,
                "artifact_size_bytes": 300_000,
            },
        ]
        metadata = {
            "environment": "test",
            "platform_name": "win11",
            "selected_targets": ["fast", "slow"],
        }

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            formatter.write(
                root / "raw",
                performance,
                memory,
                compilation,
                metadata,
                root / "aggregated",
                root / "reports",
            )
            with (root / "aggregated" / "scorecard_summary.csv").open(encoding="utf-8") as handle:
                rows = list(csv.DictReader(handle))

        self.assertEqual(len(rows), 2)
        self.assertEqual(rows[0]["target_id"], "fast")
        self.assertEqual(rows[0]["winner"], "True")
        self.assertGreater(float(rows[0]["overall_score"]), float(rows[1]["overall_score"]))
        self.assertAlmostEqual(float(rows[1]["slowdown_vs_winner_percent"]), 100.0, places=4)


if __name__ == "__main__":
    unittest.main()
