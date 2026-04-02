from __future__ import annotations

import subprocess
import time
from pathlib import Path
from typing import Any

from benchmarks.infrastructure.utils import sanitize_preview


class CompilationAnalyzer:
    def measure(
        self,
        command: list[str] | None,
        cwd: Path,
        env: dict[str, str] | None = None,
    ) -> dict[str, Any] | None:
        if not command:
            return None

        start = time.perf_counter_ns()
        completed = subprocess.run(
            command,
            cwd=cwd,
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )
        duration_ms = (time.perf_counter_ns() - start) / 1_000_000
        return {
            "command": command,
            "duration_ms": round(duration_ms, 3),
            "return_code": completed.returncode,
            "stderr_preview": sanitize_preview(completed.stderr),
        }

