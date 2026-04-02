from __future__ import annotations

import os
from pathlib import Path

from benchmarks.harness.base_harness import BaseHarness, PreparedBenchmark
from benchmarks.infrastructure.utils import resolve_command_path


class RustHarness(BaseHarness):
    language = "rust"
    suffixes = (".rs",)

    def prepare(
        self,
        source: Path,
        build_target: Path,
        target_id: str,
        target_spec: dict[str, object],
    ) -> PreparedBenchmark:
        binary = build_target.with_suffix(".exe" if os.name == "nt" else "")
        rustc = str(resolve_command_path(str(self.environment["rustc"])) or self.environment["rustc"])
        rust_flags = [str(flag) for flag in target_spec.get("rust_flags", ["-C", "opt-level=3"])]
        compile_command = [rustc, *rust_flags, "-o", str(binary), str(source)]
        return PreparedBenchmark(
            target_id=target_id,
            target_name=str(target_spec.get("name", target_id)),
            language=self.language,
            backend="native",
            compiler=str(target_spec.get("compiler", rustc)),
            call_cache_enabled=False,
            compile_command=compile_command,
            run_command=[str(binary)],
            artifact_path=binary,
            runtime_executable=binary,
        )
