"""GPU Matrix Multiplication — Numba CUDA JIT."""
import sys
import time
import math

DEFAULT_SIZE = 512
TPB = 16  # threads per block


def matmul_numba(size: int) -> dict:
    import numpy as np
    from numba import cuda

    @cuda.jit
    def matmul_kernel(A, B, C):
        row, col = cuda.grid(2)
        if row < C.shape[0] and col < C.shape[1]:
            tmp = 0.0
            for k in range(A.shape[1]):
                tmp += A[row, k] * B[k, col]
            C[row, col] = tmp

    a = np.random.randn(size, size).astype(np.float32)
    b = np.random.randn(size, size).astype(np.float32)
    c = np.zeros((size, size), dtype=np.float32)

    d_a = cuda.to_device(a)
    d_b = cuda.to_device(b)
    d_c = cuda.to_device(c)

    threads_per_block = (TPB, TPB)
    blocks_per_grid_x = math.ceil(size / TPB)
    blocks_per_grid_y = math.ceil(size / TPB)
    blocks_per_grid = (blocks_per_grid_x, blocks_per_grid_y)

    # Warmup
    for _ in range(3):
        matmul_kernel[blocks_per_grid, threads_per_block](d_a, d_b, d_c)
    cuda.synchronize()

    # Measure
    timings = []
    for _ in range(7):
        cuda.synchronize()
        t0 = time.perf_counter_ns()
        matmul_kernel[blocks_per_grid, threads_per_block](d_a, d_b, d_c)
        cuda.synchronize()
        t1 = time.perf_counter_ns()
        timings.append(t1 - t0)

    d_c.copy_to_host(c)
    return {
        "framework": "numba_cuda",
        "device": "cuda",
        "size": size,
        "shape": f"{size}x{size}",
        "timings_ns": timings,
        "median_ns": sorted(timings)[len(timings) // 2],
        "checksum": float(c.sum()),
    }


if __name__ == "__main__":
    import json
    size = int(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_SIZE
    print(json.dumps([matmul_numba(size)], indent=2))
