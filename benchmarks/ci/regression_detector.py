from __future__ import annotations

if __package__ in {None, ""}:  # pragma: no cover
    import sys
    from pathlib import Path

    sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

import argparse
import json
from pathlib import Path


def build_index(rows: list[dict]) -> dict[tuple[str, str, str], dict]:
    return {
        (row["suite"], row["case"], row["language"]): row
        for row in rows
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Detect benchmark regressions.")
    parser.add_argument("--baseline", required=True, help="Baseline statistical_analysis.json")
    parser.add_argument("--current", required=True, help="Current statistical_analysis.json")
    parser.add_argument("--threshold", type=float, default=5.0, help="Regression threshold percent")
    args = parser.parse_args()

    baseline_rows = json.loads(Path(args.baseline).read_text(encoding="utf-8"))
    current_rows = json.loads(Path(args.current).read_text(encoding="utf-8"))
    baseline_index = build_index(baseline_rows)
    current_index = build_index(current_rows)

    regressions: list[str] = []
    for key, current in current_index.items():
        baseline = baseline_index.get(key)
        if not baseline:
            continue
        baseline_mean = baseline.get("mean_ms")
        current_mean = current.get("mean_ms")
        if not baseline_mean or current_mean is None:
            continue
        delta_percent = ((current_mean - baseline_mean) / baseline_mean) * 100
        if delta_percent > args.threshold:
            regressions.append(
                f"{key[0]}/{key[1]}/{key[2]} regressed by {delta_percent:.2f}%"
            )

    if regressions:
        for regression in regressions:
            print(regression)
        return 1

    print("No regressions detected.")
    return 0


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())

