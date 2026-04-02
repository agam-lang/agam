from __future__ import annotations

import os
from pathlib import Path

from benchmarks.harness.base_harness import BaseHarness, PreparedBenchmark


class GoHarness(BaseHarness):
    language = "go"
    suffixes = (".go",)

    def prepare(
        self,
        source: Path,
        build_target: Path,
        target_id: str,
        target_spec: dict[str, object],
    ) -> PreparedBenchmark:
        binary = build_target.with_suffix(".exe" if os.name == "nt" else "")
        go = str(self.environment["go"])
        compile_command = [go, "build", "-o", str(binary), str(source)]
        return PreparedBenchmark(
            target_id=target_id,
            target_name=str(target_spec.get("name", target_id)),
            language=self.language,
            backend="native",
            compiler=str(target_spec.get("compiler", go)),
            call_cache_enabled=False,
            compile_command=compile_command,
            run_command=[str(binary)],
            artifact_path=binary,
            runtime_executable=binary,
        )
