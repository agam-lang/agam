from __future__ import annotations

from pathlib import Path

from benchmarks.harness.base_harness import BaseHarness, PreparedBenchmark


class AgamHarness(BaseHarness):
    language = "agam"
    suffixes = (".agam",)

    def prepare(self, source: Path, build_target: Path) -> PreparedBenchmark:
        driver = self.environment["agam_driver"]
        compile_command = [
            *driver,
            "build",
            str(source),
            "--backend",
            "llvm",
            "-O",
            "3",
        ]
        run_command = [
            *driver,
            "run",
            str(source),
            "--fast",
        ]
        return PreparedBenchmark(compile_command=compile_command, run_command=run_command)

