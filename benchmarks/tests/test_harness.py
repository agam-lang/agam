from __future__ import annotations

import json
import unittest
from pathlib import Path

from benchmarks.infrastructure.benchmark_harness import BenchmarkWorkspace


class HarnessTests(unittest.TestCase):
    def test_harness_selection(self) -> None:
        workspace = BenchmarkWorkspace()
        self.assertEqual(workspace.harness_for(Path("sample.agam")).language, "agam")
        self.assertEqual(workspace.harness_for(Path("sample.py")).language, "python")
        self.assertEqual(workspace.harness_for(Path("sample.rs")).language, "rust")
        self.assertEqual(workspace.harness_for(Path("sample.cpp")).language, "cpp")

    def test_dry_run_writes_metadata(self) -> None:
        workspace = BenchmarkWorkspace()
        result = workspace.run(
            suites=["01_algorithms"],
            include_comparisons=True,
            language_filters={"python"},
            target_filters=["python_cpython"],
            match_filters=["fibonacci"],
            warmups=0,
            runs=1,
            max_benchmarks=1,
            dry_run=True,
        )
        self.assertIn("run_root", result)
        self.assertEqual(result["performance_rows"], 0)
        self.assertEqual(result["memory_rows"], 1)

    def test_runtime_rows_capture_stdout_debugging_fields(self) -> None:
        workspace = BenchmarkWorkspace()
        result = workspace.run(
            suites=["01_algorithms"],
            include_comparisons=True,
            language_filters={"python"},
            target_filters=["python_cpython"],
            match_filters=["fibonacci"],
            warmups=0,
            runs=1,
            max_benchmarks=1,
            dry_run=False,
        )
        performance_path = Path(result["run_root"]) / "performance.json"
        rows = json.loads(performance_path.read_text(encoding="utf-8"))
        self.assertEqual(len(rows), 1)
        self.assertIn("stdout_preview", rows[0])
        self.assertIn("stderr_preview", rows[0])
        self.assertIn("stdout_hashes", rows[0])
        self.assertFalse(rows[0]["stdout_mismatch_detected"])


if __name__ == "__main__":
    unittest.main()
