"""GPU Reduction Sum — NumPy CPU baseline."""
import sys, time, json
DEFAULT_SIZE = 10_000_000
def reduction_numpy(size: int) -> dict:
    import numpy as np
    a = np.random.randn(size).astype(np.float32)
    for _ in range(3): _ = np.sum(a)
    timings = []
    for _ in range(7):
        t0 = time.perf_counter_ns()
        s = np.sum(a)
        timings.append(time.perf_counter_ns() - t0)
    return {"framework": "numpy", "device": "cpu", "size": size,
            "timings_ns": timings, "median_ns": sorted(timings)[3],
            "checksum": float(s)}
if __name__ == "__main__":
    size = int(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_SIZE
    print(json.dumps([reduction_numpy(size)], indent=2))
