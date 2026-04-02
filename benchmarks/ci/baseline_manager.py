from __future__ import annotations

if __package__ in {None, ""}:  # pragma: no cover
    import sys
    from pathlib import Path

    sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

import argparse
import json
from pathlib import Path


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Manage benchmark baselines.")
    subparsers = parser.add_subparsers(dest="command", required=True)

    write_parser = subparsers.add_parser("write", help="Write a baseline snapshot")
    write_parser.add_argument("--summary", required=True, help="Path to statistical_analysis.json")
    write_parser.add_argument("--output", required=True, help="Baseline output path")

    show_parser = subparsers.add_parser("show", help="Print a baseline snapshot")
    show_parser.add_argument("--baseline", required=True, help="Baseline JSON path")
    return parser


def main() -> int:
    args = build_parser().parse_args()
    if args.command == "write":
        source = Path(args.summary)
        output = Path(args.output)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(source.read_text(encoding="utf-8"), encoding="utf-8")
        print(output)
        return 0

    baseline = Path(args.baseline)
    print(json.dumps(json.loads(baseline.read_text(encoding="utf-8")), indent=2))
    return 0


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())

