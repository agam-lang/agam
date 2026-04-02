from __future__ import annotations

import unittest

from benchmarks.infrastructure.statistical_analyzer import StatisticalAnalyzer


class StatisticalAnalyzerTests(unittest.TestCase):
    def test_summary_contains_core_fields(self) -> None:
        summary = StatisticalAnalyzer().summarize([10.0, 12.0, 11.0])
        self.assertEqual(summary["sample_count"], 3)
        self.assertAlmostEqual(summary["mean_ms"], 11.0)
        self.assertAlmostEqual(summary["median_ms"], 11.0)

    def test_summary_includes_baseline_delta(self) -> None:
        summary = StatisticalAnalyzer().summarize([11.0, 11.0, 11.0], [10.0, 10.0, 10.0])
        self.assertAlmostEqual(summary["delta_percent"], 10.0)
        self.assertAlmostEqual(summary["speedup_factor"], 10.0 / 11.0, places=6)


if __name__ == "__main__":
    unittest.main()
