from __future__ import annotations

if __package__ in {None, ""}:  # pragma: no cover
    import sys
    from pathlib import Path

    sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

import argparse
import subprocess
from pathlib import Path

from benchmarks.infrastructure.utils import REPO_ROOT, resolve_agam_driver_command, resolve_command_path


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Compile an Agam C-backend benchmark to a native binary.")
    parser.add_argument("--source", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--opt-level", type=int, default=3)
    parser.add_argument("--call-cache", action="store_true")
    return parser


def main() -> int:
    args = build_parser().parse_args()
    source = Path(args.source)
    output = Path(args.output)
    generated_c = output.with_suffix(".c")
    driver = resolve_agam_driver_command(["cargo", "run", "-p", "agam_driver", "--"])

    build_command = [
        *driver,
        "build",
        str(source),
        "--backend",
        "c",
        "-O",
        str(args.opt_level),
        "--output",
        str(generated_c),
    ]
    if args.call_cache:
        build_command.append("--call-cache")

    build = subprocess.run(build_command, cwd=REPO_ROOT, check=False)
    if build.returncode != 0:
        return build.returncode

    clang = resolve_command_path("clang")
    if clang is None:
        raise SystemExit("clang could not be resolved for the Agam C-backend benchmark")

    compile_command = [str(clang), "-O3", "-o", str(output), str(generated_c)]
    compiled = subprocess.run(compile_command, cwd=REPO_ROOT, check=False)
    return compiled.returncode


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
