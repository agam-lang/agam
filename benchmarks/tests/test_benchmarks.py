from __future__ import annotations

import unittest

from benchmarks.infrastructure.utils import BENCHMARK_ROOT, SUITE_ROOT, discover_benchmarks


class BenchmarkWorkspaceShapeTests(unittest.TestCase):
    def test_workspace_docs_exist(self) -> None:
        self.assertTrue((BENCHMARK_ROOT / "README.md").is_file())
        self.assertTrue((BENCHMARK_ROOT / "METHODOLOGY.md").is_file())
        self.assertTrue((BENCHMARK_ROOT / "COVERAGE_MATRIX.md").is_file())
        self.assertTrue((BENCHMARK_ROOT / "results" / "README.md").is_file())

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

    def test_cpp_comparisons_exist(self) -> None:
        self.assertTrue((SUITE_ROOT / "01_algorithms" / "comparisons" / "fibonacci.cpp").is_file())
        self.assertTrue(
            (SUITE_ROOT / "02_numerical_computation" / "comparisons" / "matrix_multiply.cpp").is_file()
        )

    def test_workspace_keeps_30_plus_agam_benchmarks(self) -> None:
        agam_sources = discover_benchmarks(language_filters={"agam"})
        self.assertGreaterEqual(len(agam_sources), 35)

    def test_workspace_keeps_35_plus_comparison_sources(self) -> None:
        comparison_sources = [
            path
            for path in discover_benchmarks(include_comparisons=True)
            if "comparisons" in path.parts
        ]
        self.assertGreaterEqual(len(comparison_sources), 35)

    def test_cross_language_workloads_exist_for_new_constraint_slice(self) -> None:
        expected = {
            "01_algorithms": {"edit_distance"},
            "02_numerical_computation": {"polynomial_eval"},
            "03_data_structures": {"ring_buffer"},
            "06_string_processing": {"token_frequency"},
            "07_io_operations": {"csv_scanning"},
        }
        expected_suffixes = {".c", ".cpp", ".go", ".py", ".rs"}
        for suite, workload_names in expected.items():
            comparison_dir = SUITE_ROOT / suite / "comparisons"
            by_stem: dict[str, set[str]] = {}
            for path in comparison_dir.iterdir():
                if not path.is_file() or path.suffix not in expected_suffixes:
                    continue
                by_stem.setdefault(path.stem, set()).add(path.suffix)
            for workload_name in workload_names:
                self.assertEqual(by_stem.get(workload_name), expected_suffixes)

    def test_results_readme_tracks_result_status_by_workload(self) -> None:
        text = (BENCHMARK_ROOT / "results" / "README.md").read_text(encoding="utf-8")
        self.assertIn("## Comparison-Ready Result Coverage", text)
        self.assertIn("## Agam-Only Call-Cache Result Coverage", text)
        for workload_name in (
            "fibonacci",
            "edit_distance",
            "matrix_multiply",
            "polynomial_eval",
            "ring_buffer",
            "tensor_matmul",
            "token_frequency",
            "csv_scanning",
            "call_cache_hotset",
            "call_cache_mixed_locality",
            "call_cache_phase_shift",
            "call_cache_unique_inputs",
        ):
            self.assertIn(f"`{workload_name}`", text)
        self.assertIn("`measured snapshot`", text)
        self.assertIn("`dry-run validated`", text)
        self.assertIn("`source-present only`", text)

    def test_coverage_matrix_tracks_broader_methodology_backlog(self) -> None:
        text = (BENCHMARK_ROOT / "COVERAGE_MATRIX.md").read_text(encoding="utf-8")
        self.assertIn("planned or future workloads tracked below: `92`", text)
        self.assertIn("total workload slots tracked in this matrix: `130`", text)
        for workload_name in (
            "sha3_hash",
            "ecdsa_sign_verify",
            "minimax_tree_search",
            "finite_element_mesh",
            "page_rank",
            "blocked_tiled_matrix_multiply",
            "gpu_ray_tracing",
            "llm_prefill",
            "random_block_io_4k",
            "http_get_flood",
            "nvme_direct_storage",
            "quantum_fourier_transform_sim",
        ):
            self.assertIn(f"`{workload_name}`", text)

    def test_jit_suite_keeps_multiple_call_cache_shapes(self) -> None:
        jit_sources = discover_benchmarks(
            suite_filters=["08_jit_optimization"],
            language_filters={"agam"},
        )
        self.assertGreaterEqual(len(jit_sources), 7)


if __name__ == "__main__":
    unittest.main()
