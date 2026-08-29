"""GPU Elementwise FMA — NumPy CPU baseline."""
import sys, time, json
DEFAULT_SIZE = 10_000_000
def elementwise_numpy(size: int) -> dict:
    import numpy as np
    a = np.random.randn(size).astype(np.float32)
    b = np.random.randn(size).astype(np.float32)
    c = np.random.randn(size).astype(np.float32)
    for _ in range(3): _ = c + a * b
    timings = []
    for _ in range(7):
        t0 = time.perf_counter_ns()
        result = c + a * b
        timings.append(time.perf_counter_ns() - t0)
    return {"framework": "numpy", "device": "cpu", "size": size, "op": "fma",
            "timings_ns": timings, "median_ns": sorted(timings)[3],
            "checksum": float(result.sum())}
if __name__ == "__main__":
    size = int(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_SIZE
    print(json.dumps([elementwise_numpy(size)], indent=2))
