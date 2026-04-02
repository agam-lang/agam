from __future__ import annotations

import unittest
from pathlib import Path

from benchmarks.infrastructure.benchmark_harness import BenchmarkWorkspace


class HarnessTests(unittest.TestCase):
    def test_harness_selection(self) -> None:
        workspace = BenchmarkWorkspace()
        self.assertEqual(workspace.harness_for(Path("sample.agam")).language, "agam")
        self.assertEqual(workspace.harness_for(Path("sample.py")).language, "python")
        self.assertEqual(workspace.harness_for(Path("sample.rs")).language, "rust")

    def test_dry_run_writes_metadata(self) -> None:
        workspace = BenchmarkWorkspace()
        result = workspace.run(
            suites=["01_algorithms"],
            include_comparisons=True,
            language_filters={"python"},
            warmups=0,
            runs=1,
            max_benchmarks=1,
            dry_run=True,
        )
        self.assertIn("run_root", result)
        self.assertEqual(result["performance_rows"], 0)


if __name__ == "__main__":
    unittest.main()

