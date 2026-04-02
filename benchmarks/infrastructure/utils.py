from __future__ import annotations

import hashlib
import json
import os
import platform
import re
import shutil
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

SOURCE_SUFFIX_TO_LANGUAGE = {
    ".agam": "agam",
    ".py": "python",
    ".rs": "rust",
    ".c": "c",
    ".cpp": "cpp",
    ".cc": "cpp",
    ".cxx": "cpp",
    ".go": "go",
}

COMPLEXITY_HINTS: dict[str, dict[str, str]] = {
    "fibonacci": {
        "time_complexity": "O(phi^n)",
        "space_complexity": "O(n)",
        "complexity_notes": "Naive recursive benchmark.",
    },
    "quicksort": {
        "time_complexity": "O(n log n)",
        "space_complexity": "O(log n)",
        "complexity_notes": "Synthetic partition-cost recursion shaped like quicksort.",
    },
    "binary_search": {
        "time_complexity": "O(log n)",
        "space_complexity": "O(1)",
        "complexity_notes": "Scalar search over an implicit sorted range.",
    },
    "prime_sieve": {
        "time_complexity": "O(n*sqrt(n))",
        "space_complexity": "O(1)",
        "complexity_notes": "Current benchmark uses direct primality scans, not a dense sieve array.",
    },
    "matrix_multiply": {
        "time_complexity": "O(n^3)",
        "space_complexity": "O(1)",
        "complexity_notes": "Checksum-oriented multiply without explicit output matrix storage.",
    },
    "tensor_operations": {
        "time_complexity": "O(w*h*d)",
        "space_complexity": "O(1)",
        "complexity_notes": "Dense tensor traversal benchmark.",
    },
    "fft": {
        "time_complexity": "O(n log n)",
        "space_complexity": "O(1)",
        "complexity_notes": "Butterfly-cost style FFT pressure benchmark.",
    },
    "monte_carlo_pi": {
        "time_complexity": "O(n)",
        "space_complexity": "O(1)",
        "complexity_notes": "Pseudo-random point sampling.",
    },
    "hashmap_operations": {
        "time_complexity": "O(n)",
        "space_complexity": "O(1)",
        "complexity_notes": "Probe and hash arithmetic workload.",
    },
    "btree_operations": {
        "time_complexity": "O(b^d)",
        "space_complexity": "O(d)",
        "complexity_notes": "Recursive branch-walk benchmark.",
    },
    "linked_list": {
        "time_complexity": "O(n)",
        "space_complexity": "O(1)",
        "complexity_notes": "Pointer-chasing pressure benchmark.",
    },
    "memory_allocation": {
        "time_complexity": "O(chunks*width)",
        "space_complexity": "O(1)",
        "complexity_notes": "Allocation-style loop pressure.",
    },
    "garbage_collection": {
        "time_complexity": "O(objects*generations)",
        "space_complexity": "O(1)",
        "complexity_notes": "Retention and churn simulation.",
    },
    "arc_contention": {
        "time_complexity": "O(workers*handoffs)",
        "space_complexity": "O(1)",
        "complexity_notes": "ARC/refcount handoff pressure.",
    },
    "tensor_matmul": {
        "time_complexity": "O(n^3)",
        "space_complexity": "O(1)",
        "complexity_notes": "Tensor matmul checksum benchmark.",
    },
    "convolution": {
        "time_complexity": "O(w*h)",
        "space_complexity": "O(1)",
        "complexity_notes": "Stencil-like convolution pressure benchmark.",
    },
    "autodiff": {
        "time_complexity": "O(n)",
        "space_complexity": "O(1)",
        "complexity_notes": "Forward/backward accumulation simulation.",
    },
    "softmax": {
        "time_complexity": "O(width*rounds)",
        "space_complexity": "O(1)",
        "complexity_notes": "Reduction-heavy softmax-like pass.",
    },
    "string_search": {
        "time_complexity": "O(n)",
        "space_complexity": "O(1)",
        "complexity_notes": "Scalar token-search scaffold.",
    },
    "regex_matching": {
        "time_complexity": "O(n)",
        "space_complexity": "O(1)",
        "complexity_notes": "Branch-heavy regex-style matching scaffold.",
    },
    "file_reading": {
        "time_complexity": "O(n)",
        "space_complexity": "O(1)",
        "complexity_notes": "Chunk-based I/O accounting scaffold.",
    },
    "json_parsing": {
        "time_complexity": "O(objects*fields)",
        "space_complexity": "O(1)",
        "complexity_notes": "Token and branch pressure scaffold.",
    },
    "call_cache_profile": {
        "time_complexity": "O(n)",
        "space_complexity": "O(1)",
        "complexity_notes": "Use this with call-cache on/off target pairs.",
    },
    "specialization_demo": {
        "time_complexity": "O(n)",
        "space_complexity": "O(1)",
        "complexity_notes": "Specialization branch-shape benchmark.",
    },
    "adaptive_optimization": {
        "time_complexity": "O(n)",
        "space_complexity": "O(1)",
        "complexity_notes": "Adaptive scoring branch-shape benchmark.",
    },
    "tiny_program": {
        "time_complexity": "O(1)",
        "space_complexity": "O(1)",
        "complexity_notes": "Compilation metric anchor for tiny programs.",
    },
    "medium_program": {
        "time_complexity": "O(n)",
        "space_complexity": "O(1)",
        "complexity_notes": "Compilation metric anchor for medium programs.",
    },
    "large_program": {
        "time_complexity": "O(stages*width)",
        "space_complexity": "O(1)",
        "complexity_notes": "Compilation metric anchor for large programs.",
    },
    "complex_generics": {
        "time_complexity": "O(n)",
        "space_complexity": "O(1)",
        "complexity_notes": "Generic-shape compile-pressure scaffold.",
    },
}


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
    paths: list[Path] = []
    suite_filter_set = set(suite_filters or [])
    for path in sorted(SUITE_ROOT.rglob("*")):
        if not path.is_file():
            continue
        language = SOURCE_SUFFIX_TO_LANGUAGE.get(path.suffix)
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
        if system == "windows":
            return "github_actions_windows"
        return "github_actions_linux"
    if os.environ.get("WSL_DISTRO_NAME"):
        return "wsl_ubuntu_24_04"
    if system == "windows":
        if platform.release() == "11":
            return "local_windows_win11"
        return "local_windows_win11"
    return "local_linux_native"


def host_metadata() -> dict[str, Any]:
    return {
        "platform": platform.platform(),
        "system": platform.system(),
        "release": platform.release(),
        "machine": platform.machine(),
        "python_version": platform.python_version(),
        "processor": platform.processor(),
    }


def complexity_hint_for(path: Path) -> dict[str, str | None]:
    return COMPLEXITY_HINTS.get(
        path.stem,
        {
            "time_complexity": None,
            "space_complexity": None,
            "complexity_notes": None,
        },
    )


def file_size_bytes(path: Path | None) -> int | None:
    if path is None or not path.exists() or not path.is_file():
        return None
    return path.stat().st_size


def resolve_command_path(command: str | None) -> Path | None:
    if not command:
        return None
    resolved = shutil.which(command)
    return Path(resolved) if resolved else None


def parse_csv_arguments(values: list[str] | None) -> list[str] | None:
    if not values:
        return None
    parsed: list[str] = []
    for raw in values:
        parsed.extend(part.strip() for part in raw.split(",") if part.strip())
    return parsed or None
