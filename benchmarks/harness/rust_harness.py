from __future__ import annotations

import os
from pathlib import Path

from benchmarks.harness.base_harness import BaseHarness, PreparedBenchmark


class RustHarness(BaseHarness):
    language = "rust"
    suffixes = (".rs",)

    def prepare(self, source: Path, build_target: Path) -> PreparedBenchmark:
        binary = build_target.with_suffix(".exe" if self._is_windows() else "")
        rustc = self.environment["rustc"]
        compile_command = [rustc, "-C", "opt-level=3", "-o", str(binary), str(source)]
        run_command = [str(binary)]
        return PreparedBenchmark(compile_command=compile_command, run_command=run_command)

    def _is_windows(self) -> bool:
        return os.name == "nt"
