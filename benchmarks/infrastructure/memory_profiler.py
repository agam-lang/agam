from __future__ import annotations

import ctypes
import os
import time
from pathlib import Path
from subprocess import Popen
from typing import Any

from benchmarks.infrastructure.cpu_profiler import CpuTopology
from benchmarks.infrastructure.utils import file_size_bytes


class PROCESS_MEMORY_COUNTERS(ctypes.Structure):
    _fields_ = [
        ("cb", ctypes.c_ulong),
        ("PageFaultCount", ctypes.c_ulong),
        ("PeakWorkingSetSize", ctypes.c_size_t),
        ("WorkingSetSize", ctypes.c_size_t),
        ("QuotaPeakPagedPoolUsage", ctypes.c_size_t),
        ("QuotaPagedPoolUsage", ctypes.c_size_t),
        ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t),
        ("QuotaNonPagedPoolUsage", ctypes.c_size_t),
        ("PagefileUsage", ctypes.c_size_t),
        ("PeakPagefileUsage", ctypes.c_size_t),
    ]


class MemoryProfiler:
    """Runtime memory and static space-profile collection."""

    def __init__(self, poll_interval: float = 0.01) -> None:
        self.poll_interval = poll_interval

    def capture_peak_rss_bytes(self, process: Popen[bytes]) -> int | None:
        if os.name == "nt":
            return self._capture_peak_windows(process)
        return self._capture_peak_procfs(process)

    def build_space_profile(
        self,
        source_path: Path,
        artifact_path: Path | None,
        runtime_executable: Path | None,
        cpu_topology: CpuTopology,
        peak_rss_bytes: int | None,
    ) -> dict[str, Any]:
        source_size = file_size_bytes(source_path)
        artifact_size = file_size_bytes(artifact_path)
        runtime_size = file_size_bytes(runtime_executable)

        ssd_footprint = artifact_size
        if ssd_footprint is None:
            ssd_footprint = (source_size or 0) + (runtime_size or 0) or None
        l3_pressure = None
        if peak_rss_bytes is not None and cpu_topology.l3_cache_bytes:
            l3_pressure = round(peak_rss_bytes / cpu_topology.l3_cache_bytes, 6)

        return {
            "source_size_bytes": source_size,
            "artifact_size_bytes": artifact_size,
            "runtime_executable_size_bytes": runtime_size,
            "ssd_footprint_bytes": ssd_footprint,
            "peak_rss_bytes": peak_rss_bytes,
            "l1_data_cache_bytes": cpu_topology.l1_data_cache_bytes,
            "l2_cache_bytes": cpu_topology.l2_cache_bytes,
            "l3_cache_bytes": cpu_topology.l3_cache_bytes,
            "cache_line_bytes": cpu_topology.cache_line_bytes,
            "pointer_width_bits": cpu_topology.pointer_width_bits,
            "simd_register_width_bytes": cpu_topology.simd_register_width_bytes,
            "general_register_bytes_estimate": cpu_topology.general_register_bytes_estimate,
            "simd_register_file_bytes_estimate": cpu_topology.simd_register_file_bytes_estimate,
            "register_file_bytes_estimate": cpu_topology.register_file_bytes_estimate,
            "l3_pressure_ratio_estimate": l3_pressure,
        }

    def _capture_peak_procfs(self, process: Popen[bytes]) -> int | None:
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

    def _capture_peak_windows(self, process: Popen[bytes]) -> int | None:
        PROCESS_QUERY_INFORMATION = 0x0400
        PROCESS_VM_READ = 0x0010

        open_process = ctypes.windll.kernel32.OpenProcess
        close_handle = ctypes.windll.kernel32.CloseHandle
        get_memory_info = ctypes.windll.psapi.GetProcessMemoryInfo

        handle = open_process(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, False, process.pid)
        if not handle:
            process.wait()
            return None

        peak = 0
        try:
            counters = PROCESS_MEMORY_COUNTERS()
            counters.cb = ctypes.sizeof(PROCESS_MEMORY_COUNTERS)
            while process.poll() is None:
                if get_memory_info(handle, ctypes.byref(counters), counters.cb):
                    peak = max(peak, int(counters.PeakWorkingSetSize or counters.WorkingSetSize))
                time.sleep(self.poll_interval)
            if get_memory_info(handle, ctypes.byref(counters), counters.cb):
                peak = max(peak, int(counters.PeakWorkingSetSize or counters.WorkingSetSize))
        finally:
            close_handle(handle)

        return peak or None

    @staticmethod
    def _read_rss_kib(status_path: Path) -> int:
        try:
            for line in status_path.read_text(encoding="utf-8").splitlines():
                if line.startswith("VmRSS:"):
                    return int(line.split()[1])
        except (FileNotFoundError, PermissionError, ValueError):
            return 0
        return 0
