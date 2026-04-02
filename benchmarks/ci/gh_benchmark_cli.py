from __future__ import annotations

if __package__ in {None, ""}:  # pragma: no cover
    import sys
    from pathlib import Path

    sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

import argparse
import subprocess

from benchmarks.infrastructure.utils import BENCHMARK_ROOT, load_yaml_like


CONFIG = load_yaml_like(BENCHMARK_ROOT / "ci" / "benchmark_ci.yaml")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="GitHub CLI integration for benchmark workflows.")
    subparsers = parser.add_subparsers(dest="command", required=True)

    run_parser = subparsers.add_parser("run", help="Dispatch the benchmark workflow")
    run_parser.add_argument("--ref", default="main", help="Git ref to benchmark")
    run_parser.add_argument("--suite", action="append", help="Optional suite filter")

    list_parser = subparsers.add_parser("list", help="List recent benchmark workflow runs")
    list_parser.add_argument("--limit", type=int, default=10, help="Number of runs to show")

    download_parser = subparsers.add_parser("download", help="Download workflow artifacts")
    download_parser.add_argument("--run-id", required=True, help="Workflow run id")
    download_parser.add_argument(
        "--dir",
        default="benchmarks/results/github",
        help="Directory where artifacts should be downloaded",
    )
    return parser


def main() -> int:
    args = build_parser().parse_args()
    workflow_file = CONFIG["workflow"]["file"]

    if args.command == "run":
        command = [
            "gh",
            "workflow",
            "run",
            workflow_file,
            "--ref",
            args.ref,
        ]
        if args.suite:
            command.extend(["-f", f"suites={','.join(args.suite)}"])
        return subprocess.run(command, check=False).returncode

    if args.command == "list":
        command = [
            "gh",
            "run",
            "list",
            "--workflow",
            workflow_file,
            "--limit",
            str(args.limit),
        ]
        return subprocess.run(command, check=False).returncode

    command = [
        "gh",
        "run",
        "download",
        args.run_id,
        "--dir",
        args.dir,
    ]
    return subprocess.run(command, check=False).returncode


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())

