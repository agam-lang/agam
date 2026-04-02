from __future__ import annotations

import os
from pathlib import Path

from benchmarks.harness.base_harness import BaseHarness, PreparedBenchmark
from benchmarks.infrastructure.utils import resolve_command_path


class CHarness(BaseHarness):
    language = "c"
    suffixes = (".c",)

    def prepare(
        self,
        source: Path,
        build_target: Path,
        target_id: str,
        target_spec: dict[str, object],
    ) -> PreparedBenchmark:
        binary = build_target.with_suffix(".exe" if os.name == "nt" else "")
        compiler_key = str(target_spec.get("compiler_key", "c_compiler"))
        compiler = str(resolve_command_path(str(self.environment[compiler_key])) or self.environment[compiler_key])
        compile_args = [str(flag) for flag in target_spec.get("compile_args", ["-O3"])]
        compile_command = [compiler, *compile_args, "-o", str(binary), str(source)]
        return PreparedBenchmark(
            target_id=target_id,
            target_name=str(target_spec.get("name", target_id)),
            language=self.language,
            backend="native",
            compiler=str(target_spec.get("compiler", compiler)),
            call_cache_enabled=False,
            compile_command=compile_command,
            run_command=[str(binary)],
            artifact_path=binary,
            runtime_executable=binary,
        )
