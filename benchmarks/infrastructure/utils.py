from __future__ import annotations

import hashlib
import json
import os
import platform
import re
from datetime import UTC, datetime
from pathlib import Path
from typing import Any


try:
    import yaml  # type: ignore
except ImportError:  # pragma: no cover - optional dependency
    yaml = None


REPO_ROOT = Path(__file__).resolve().parents[2]
BENCHMARK_ROOT = REPO_ROOT / "benchmarks"
SUITE_ROOT = BENCHMARK_ROOT / "benchmarks"
RESULT_ROOT = BENCHMARK_ROOT / "results"
CONFIG_ROOT = BENCHMARK_ROOT / "config"


def ensure_directory(path: Path) -> Path:
    path.mkdir(parents=True, exist_ok=True)
    return path


def load_yaml_like(path: Path) -> Any:
    text = path.read_text(encoding="utf-8")
    if yaml is not None:
        return yaml.safe_load(text)
    return json.loads(text)


def timestamp_label() -> str:
    return datetime.now(UTC).strftime("%Y-%m-%d_%H-%M-%S")


def benchmark_name_for(path: Path) -> str:
    relative = path.relative_to(SUITE_ROOT)
    return relative.as_posix().replace("/", "__").replace(".", "_")


def discover_benchmarks(
    suite_filters: list[str] | None = None,
    include_comparisons: bool = False,
    language_filters: set[str] | None = None,
) -> list[Path]:
    suffix_to_language = {
        ".agam": "agam",
        ".py": "python",
        ".rs": "rust",
        ".c": "c",
        ".go": "go",
    }
    paths: list[Path] = []
    suite_filter_set = set(suite_filters or [])
    for path in sorted(SUITE_ROOT.rglob("*")):
        if not path.is_file():
            continue
        language = suffix_to_language.get(path.suffix)
        if language is None:
            continue
        if not include_comparisons and "comparisons" in path.parts:
            continue
        suite_name = path.relative_to(SUITE_ROOT).parts[0]
        if suite_filter_set and suite_name not in suite_filter_set:
            continue
        if language_filters and language not in language_filters:
            continue
        paths.append(path)
    return paths


def sha256_text(value: str) -> str:
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


def sanitize_preview(value: str, limit: int = 240) -> str:
    compact = re.sub(r"\s+", " ", value).strip()
    return compact[:limit]


def current_environment_name() -> str:
    system = platform.system().lower()
    if os.environ.get("GITHUB_ACTIONS") == "true":
        return "github_actions_linux"
    if system == "windows":
        return "local_windows"
    return "local_linux"


def host_metadata() -> dict[str, Any]:
    return {
        "platform": platform.platform(),
        "system": platform.system(),
        "release": platform.release(),
        "machine": platform.machine(),
        "python_version": platform.python_version(),
        "processor": platform.processor(),
    }

