from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass(slots=True)
class PreparedBenchmark:
    compile_command: list[str] | None
    run_command: list[str]


class BaseHarness:
    language = "unknown"
    suffixes: tuple[str, ...] = ()

    def __init__(self, environment: dict[str, Any], targets: dict[str, Any]) -> None:
        self.environment = environment
        self.targets = targets

    def can_handle(self, source: Path) -> bool:
        return source.suffix in self.suffixes

    def prepare(self, source: Path, build_target: Path) -> PreparedBenchmark:
        raise NotImplementedError

