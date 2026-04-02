from __future__ import annotations

from pathlib import Path

from benchmarks.harness.base_harness import BaseHarness, PreparedBenchmark


class PythonHarness(BaseHarness):
    language = "python"
    suffixes = (".py",)

    def prepare(self, source: Path, build_target: Path) -> PreparedBenchmark:
        return PreparedBenchmark(
            compile_command=None,
            run_command=[self.environment["python"], str(source)],
        )

