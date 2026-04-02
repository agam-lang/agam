#!/usr/bin/env bash
set -euo pipefail

python -m benchmarks.infrastructure.benchmark_harness "$@"

