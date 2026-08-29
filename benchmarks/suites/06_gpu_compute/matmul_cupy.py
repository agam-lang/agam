"""GPU Matrix Multiplication — CuPy (CUDA-native NumPy)."""
import sys
import time

DEFAULT_SIZE = 512


def matmul_cupy(size: int) -> dict:
    import cupy as cp

    a = cp.random.randn(size, size, dtype=cp.float32)
    b = cp.random.randn(size, size, dtype=cp.float32)

    # Warmup
    for _ in range(3):
        _ = cp.dot(a, b)
    cp.cuda.Device().synchronize()

    # Measure
    timings = []
    for _ in range(7):
        cp.cuda.Device().synchronize()
        t0 = time.perf_counter_ns()
        c = cp.dot(a, b)
        cp.cuda.Device().synchronize()
        t1 = time.perf_counter_ns()
        timings.append(t1 - t0)

    return {
        "framework": "cupy",
        "device": "cuda",
        "size": size,
        "shape": f"{size}x{size}",
        "timings_ns": timings,
        "median_ns": sorted(timings)[len(timings) // 2],
        "checksum": float(cp.asnumpy(c.sum())),
    }


if __name__ == "__main__":
    import json
    size = int(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_SIZE
    print(json.dumps([matmul_cupy(size)], indent=2))
