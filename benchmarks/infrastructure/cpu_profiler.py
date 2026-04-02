from __future__ import annotations

import json
import os
import platform
import subprocess
from dataclasses import asdict, dataclass
from pathlib import Path


@dataclass(slots=True)
class CpuSample:
    wall_time_ms: float
    return_code: int


@dataclass(slots=True)
class CpuTopology:
    platform_name: str
    arch: str
    physical_cores: int
    logical_cores: int
    l1_data_cache_bytes: int
    l2_cache_bytes: int
    l3_cache_bytes: int
    cache_line_bytes: int
    pointer_width_bits: int
    simd_register_width_bytes: int
    general_register_bytes_estimate: int
    simd_register_file_bytes_estimate: int
    register_file_bytes_estimate: int

    def to_dict(self) -> dict[str, int | str]:
        return asdict(self)


class CpuProfiler:
    def detect_topology(self) -> CpuTopology:
        system = platform.system().lower()
        logical_cores = os.cpu_count() or 1
        physical_cores = max(1, logical_cores // 2)
        l1 = 32 * 1024
        l2 = 256 * 1024
        l3 = 8 * 1024 * 1024
        line_size = 64

        if system == "linux":
            linux_cache = self._detect_linux_cache()
            physical_cores = linux_cache.get("physical_cores", physical_cores)
            logical_cores = linux_cache.get("logical_cores", logical_cores)
            l1 = linux_cache.get("l1_data_cache_bytes", l1)
            l2 = linux_cache.get("l2_cache_bytes", l2)
            l3 = linux_cache.get("l3_cache_bytes", l3)
            line_size = linux_cache.get("cache_line_bytes", line_size)
        elif system == "windows":
            windows_cache = self._detect_windows_cache()
            physical_cores = windows_cache.get("physical_cores", physical_cores)
            logical_cores = windows_cache.get("logical_cores", logical_cores)
            l2 = windows_cache.get("l2_cache_bytes", l2)
            l3 = windows_cache.get("l3_cache_bytes", l3)

        arch = platform.machine().lower() or platform.machine()
        pointer_width_bits = 8 * int((64 if "64" in arch else 32) / 8)
        simd_width, general_register_bytes, simd_register_bytes = self._register_profile_for_arch(
            arch,
            pointer_width_bits,
        )

        return CpuTopology(
            platform_name=platform.platform(),
            arch=arch,
            physical_cores=physical_cores,
            logical_cores=logical_cores,
            l1_data_cache_bytes=l1,
            l2_cache_bytes=l2,
            l3_cache_bytes=l3,
            cache_line_bytes=line_size,
            pointer_width_bits=pointer_width_bits,
            simd_register_width_bytes=simd_width,
            general_register_bytes_estimate=general_register_bytes,
            simd_register_file_bytes_estimate=simd_register_bytes,
            register_file_bytes_estimate=general_register_bytes + simd_register_bytes,
        )

    def _detect_linux_cache(self) -> dict[str, int]:
        cache_root = Path("/sys/devices/system/cpu/cpu0/cache")
        result: dict[str, int] = {
            "logical_cores": os.cpu_count() or 1,
            "physical_cores": max(1, (os.cpu_count() or 1) // 2),
        }
        if not cache_root.exists():
            return result

        for index_dir in cache_root.glob("index*"):
            try:
                level = int((index_dir / "level").read_text(encoding="utf-8").strip())
                cache_type = (index_dir / "type").read_text(encoding="utf-8").strip().lower()
                size = self._parse_cache_size((index_dir / "size").read_text(encoding="utf-8").strip())
                line_size = int(
                    (index_dir / "coherency_line_size").read_text(encoding="utf-8").strip()
                )
            except (FileNotFoundError, ValueError):
                continue

            result["cache_line_bytes"] = line_size
            if level == 1 and cache_type == "data":
                result["l1_data_cache_bytes"] = size
            elif level == 2:
                result["l2_cache_bytes"] = size
            elif level == 3:
                result["l3_cache_bytes"] = size
        return result

    def _detect_windows_cache(self) -> dict[str, int]:
        command = [
            "powershell",
            "-NoProfile",
            "-Command",
            (
                "$cpu = Get-CimInstance Win32_Processor | Select-Object -First 1 "
                "NumberOfCores,NumberOfLogicalProcessors,L2CacheSize,L3CacheSize; "
                "$cpu | ConvertTo-Json -Compress"
            ),
        ]
        try:
            completed = subprocess.run(
                command,
                capture_output=True,
                text=True,
                check=False,
                timeout=5,
            )
        except (OSError, subprocess.TimeoutExpired):
            return {}

        if completed.returncode != 0 or not completed.stdout.strip():
            return {}

        try:
            payload = json.loads(completed.stdout)
        except json.JSONDecodeError:
            return {}

        number_of_cores = int(payload.get("NumberOfCores") or 0)
        number_of_logical = int(payload.get("NumberOfLogicalProcessors") or 0)
        l2_kib = int(payload.get("L2CacheSize") or 0)
        l3_kib = int(payload.get("L3CacheSize") or 0)
        return {
            "physical_cores": number_of_cores or 1,
            "logical_cores": number_of_logical or max(1, number_of_cores),
            "l2_cache_bytes": l2_kib * 1024 if l2_kib else 256 * 1024,
            "l3_cache_bytes": l3_kib * 1024 if l3_kib else 8 * 1024 * 1024,
        }

    @staticmethod
    def _parse_cache_size(value: str) -> int:
        value = value.strip().upper()
        if value.endswith("K"):
            return int(value[:-1]) * 1024
        if value.endswith("M"):
            return int(value[:-1]) * 1024 * 1024
        return int(value)

    @staticmethod
    def _register_profile_for_arch(arch: str, pointer_width_bits: int) -> tuple[int, int, int]:
        pointer_bytes = pointer_width_bits // 8
        if arch in {"amd64", "x86_64"}:
            simd_width = 32
            general_register_bytes = 16 * pointer_bytes
            simd_register_bytes = 16 * simd_width
            return simd_width, general_register_bytes, simd_register_bytes
        if arch in {"arm64", "aarch64"}:
            simd_width = 16
            general_register_bytes = 31 * pointer_bytes
            simd_register_bytes = 32 * simd_width
            return simd_width, general_register_bytes, simd_register_bytes
        simd_width = 16
        general_register_bytes = 16 * pointer_bytes
        simd_register_bytes = 8 * simd_width
        return simd_width, general_register_bytes, simd_register_bytes
