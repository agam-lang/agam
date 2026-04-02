from __future__ import annotations

import os
from pathlib import Path
from shutil import which

from benchmarks.harness.base_harness import BaseHarness, PreparedBenchmark


class AgamHarness(BaseHarness):
    language = "agam"
    suffixes = (".agam",)

    def prepare(
        self,
        source: Path,
        build_target: Path,
        target_id: str,
        target_spec: dict[str, object],
    ) -> PreparedBenchmark:
        driver = self.environment["agam_driver"]
        backend = str(target_spec["backend"])
        opt_level = int(target_spec.get("optimization_level", 2))
        call_cache_enabled = bool(target_spec.get("call_cache", False))
        build_then_run = bool(target_spec.get("build_then_run", True))
        binary = build_target.with_suffix(".exe" if self._is_windows() else "")

        compile_command: list[str] | None
        run_command: list[str]
        artifact_path: Path | None

        if build_then_run:
            compile_command = [
                *driver,
                "build",
                str(source),
                "--backend",
                backend,
                "-O",
                str(opt_level),
                "--output",
                str(binary),
            ]
            if call_cache_enabled:
                compile_command.append("--call-cache")
            run_command = [str(binary)]
            artifact_path = binary
            runtime_executable = binary
        else:
            compile_command = None
            run_command = [
                *driver,
                "run",
                str(source),
                "--backend",
                backend,
                "-O",
                str(opt_level),
            ]
            if call_cache_enabled:
                run_command.append("--call-cache")
            artifact_path = None
            runtime_executable = Path(which(str(driver[0]))) if which(str(driver[0])) else None

        return PreparedBenchmark(
            target_id=target_id,
            target_name=str(target_spec.get("name", target_id)),
            language=self.language,
            backend=backend,
            compiler=str(target_spec.get("compiler", "agamc")),
            call_cache_enabled=call_cache_enabled,
            compile_command=compile_command,
            run_command=run_command,
            artifact_path=artifact_path,
            runtime_executable=runtime_executable,
            metadata={"optimization_level": opt_level},
        )

    @staticmethod
    def _is_windows() -> bool:
        return os.name == "nt"
