from __future__ import annotations

from dataclasses import dataclass


@dataclass(slots=True)
class CpuSample:
    wall_time_ms: float
    return_code: int

