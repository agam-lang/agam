from __future__ import annotations

import math
import statistics
from typing import Any


class StatisticalAnalyzer:
    def summarize(
        self,
        samples_ms: list[float],
        baseline_samples_ms: list[float] | None = None,
    ) -> dict[str, Any]:
        if not samples_ms:
            raise ValueError("samples_ms must not be empty")

        mean = statistics.fmean(samples_ms)
        median = statistics.median(samples_ms)
        minimum = min(samples_ms)
        maximum = max(samples_ms)
        stdev = statistics.stdev(samples_ms) if len(samples_ms) > 1 else 0.0
        cov = (stdev / mean) * 100 if mean else 0.0
        margin = 1.96 * (stdev / math.sqrt(len(samples_ms))) if len(samples_ms) > 1 else 0.0

        summary: dict[str, Any] = {
            "samples_ms": [round(sample, 6) for sample in samples_ms],
            "sample_count": len(samples_ms),
            "mean_ms": round(mean, 6),
            "median_ms": round(median, 6),
            "min_ms": round(minimum, 6),
            "max_ms": round(maximum, 6),
            "stdev_ms": round(stdev, 6),
            "coefficient_of_variation_percent": round(cov, 6),
            "confidence_interval_95_ms": [
                round(median - margin, 6),
                round(median + margin, 6),
            ],
        }

        if baseline_samples_ms:
            baseline_mean = statistics.fmean(baseline_samples_ms)
            delta_percent = ((mean - baseline_mean) / baseline_mean) * 100 if baseline_mean else 0.0
            summary["baseline_mean_ms"] = round(baseline_mean, 6)
            summary["delta_percent"] = round(delta_percent, 6)
            summary["speedup_factor"] = round(baseline_mean / mean, 6) if mean else None

        return summary

