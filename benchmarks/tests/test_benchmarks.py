from __future__ import annotations

import unittest

from benchmarks.infrastructure.utils import BENCHMARK_ROOT, SUITE_ROOT


class BenchmarkWorkspaceShapeTests(unittest.TestCase):
    def test_workspace_docs_exist(self) -> None:
        self.assertTrue((BENCHMARK_ROOT / "README.md").is_file())
        self.assertTrue((BENCHMARK_ROOT / "METHODOLOGY.md").is_file())

    def test_required_suite_directories_exist(self) -> None:
        expected = {
            "01_algorithms",
            "02_numerical_computation",
            "03_data_structures",
            "04_memory_intensive",
            "05_ml_primitives",
            "06_string_processing",
            "07_io_operations",
            "08_jit_optimization",
            "09_compilation_metrics",
        }
        present = {path.name for path in SUITE_ROOT.iterdir() if path.is_dir()}
        self.assertTrue(expected.issubset(present))


if __name__ == "__main__":
    unittest.main()

