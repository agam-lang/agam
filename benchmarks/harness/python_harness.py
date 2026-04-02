from __future__ import annotations

from pathlib import Path

from benchmarks.harness.base_harness import BaseHarness, PreparedBenchmark
from benchmarks.infrastructure.utils import resolve_command_path


class PythonHarness(BaseHarness):
    language = "python"
    suffixes = (".py",)

    def prepare(
        self,
        source: Path,
        build_target: Path,
        target_id: str,
        target_spec: dict[str, object],
    ) -> PreparedBenchmark:
        interpreter_key = str(target_spec.get("interpreter_key", "python"))
        interpreter = str(self.environment[interpreter_key])
        runtime_executable = resolve_command_path(interpreter)
        return PreparedBenchmark(
            target_id=target_id,
            target_name=str(target_spec.get("name", target_id)),
            language=self.language,
            backend="interpreted",
            compiler=str(target_spec.get("compiler", interpreter)),
            call_cache_enabled=False,
            compile_command=None,
            run_command=[interpreter, str(source)],
            artifact_path=None,
            runtime_executable=runtime_executable,
        )
