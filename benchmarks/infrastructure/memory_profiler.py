from __future__ import annotations

import time
from pathlib import Path
from subprocess import Popen


class MemoryProfiler:
    """Best-effort peak RSS sampler using /proc when available."""

    def __init__(self, poll_interval: float = 0.01) -> None:
        self.poll_interval = poll_interval

    def capture_peak_rss_bytes(self, process: Popen[bytes]) -> int | None:
        proc_status = Path("/proc") / str(process.pid) / "status"
        if not proc_status.exists():
            process.wait()
            return None

        peak_kib = 0
        while process.poll() is None:
            peak_kib = max(peak_kib, self._read_rss_kib(proc_status))
            time.sleep(self.poll_interval)

        peak_kib = max(peak_kib, self._read_rss_kib(proc_status))
        return peak_kib * 1024 if peak_kib else None

    @staticmethod
    def _read_rss_kib(status_path: Path) -> int:
        try:
            for line in status_path.read_text(encoding="utf-8").splitlines():
                if line.startswith("VmRSS:"):
                    return int(line.split()[1])
        except (FileNotFoundError, PermissionError, ValueError):
            return 0
        return 0

