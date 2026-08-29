"""GPU Matrix Multiplication — NumPy CPU baseline."""
import sys
import time

DEFAULT_SIZE = 512


def matmul_numpy(size: int) -> dict:
    import numpy as np

    a = np.random.randn(size, size).astype(np.float32)
    b = np.random.randn(size, size).astype(np.float32)

    # Warmup
    for _ in range(3):
        _ = np.dot(a, b)

    # Measure
    timings = []
    for _ in range(7):
        t0 = time.perf_counter_ns()
        c = np.dot(a, b)
        t1 = time.perf_counter_ns()
        timings.append(t1 - t0)

    return {
        "framework": "numpy",
        "device": "cpu",
        "size": size,
        "shape": f"{size}x{size}",
        "timings_ns": timings,
        "median_ns": sorted(timings)[len(timings) // 2],
        "checksum": float(c.sum()),
    }


if __name__ == "__main__":
    import json
    size = int(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_SIZE
    print(json.dumps([matmul_numpy(size)], indent=2))
