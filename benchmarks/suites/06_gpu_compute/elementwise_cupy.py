"""GPU Elementwise FMA — CuPy."""
import sys, time, json
DEFAULT_SIZE = 10_000_000
def elementwise_cupy(size: int) -> dict:
    import cupy as cp
    a = cp.random.randn(size, dtype=cp.float32)
    b = cp.random.randn(size, dtype=cp.float32)
    c = cp.random.randn(size, dtype=cp.float32)
    for _ in range(3): _ = c + a * b
    cp.cuda.Device().synchronize()
    timings = []
    for _ in range(7):
        cp.cuda.Device().synchronize()
        t0 = time.perf_counter_ns()
        result = c + a * b
        cp.cuda.Device().synchronize()
        timings.append(time.perf_counter_ns() - t0)
    return {"framework": "cupy", "device": "cuda", "size": size, "op": "fma",
            "timings_ns": timings, "median_ns": sorted(timings)[3],
            "checksum": float(cp.asnumpy(result.sum()))}
if __name__ == "__main__":
    size = int(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_SIZE
    print(json.dumps([elementwise_cupy(size)], indent=2))
