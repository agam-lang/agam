"""GPU Softmax — CuPy."""
import sys, time, json
DEFAULT_SIZE = 4096
DEFAULT_ROUNDS = 4000
def softmax_cupy(width: int, rounds: int) -> dict:
    import cupy as cp
    x = cp.random.randn(rounds, width, dtype=cp.float32)
    def _softmax(m):
        e = cp.exp(m - cp.max(m, axis=1, keepdims=True))
        return e / cp.sum(e, axis=1, keepdims=True)
    for _ in range(3): _ = _softmax(x)
    cp.cuda.Device().synchronize()
    timings = []
    for _ in range(7):
        cp.cuda.Device().synchronize()
        t0 = time.perf_counter_ns()
        result = _softmax(x)
        cp.cuda.Device().synchronize()
        timings.append(time.perf_counter_ns() - t0)
    return {"framework": "cupy", "device": "cuda", "width": width, "rounds": rounds,
            "op": "softmax", "timings_ns": timings, "median_ns": sorted(timings)[3],
            "checksum": float(cp.asnumpy(result.sum()))}
if __name__ == "__main__":
    w = int(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_SIZE
    r = int(sys.argv[2]) if len(sys.argv) > 2 else DEFAULT_ROUNDS
    print(json.dumps([softmax_cupy(w, r)], indent=2))
