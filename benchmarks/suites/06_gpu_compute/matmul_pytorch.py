"""GPU Matrix Multiplication — PyTorch (CPU + CUDA)."""
import sys
import time

DEFAULT_SIZE = 512


def matmul_pytorch(size: int, device: str = "cpu") -> dict:
    import torch

    a = torch.randn(size, size, device=device)
    b = torch.randn(size, size, device=device)

    # Warmup
    for _ in range(3):
        _ = torch.mm(a, b)
    if device == "cuda":
        torch.cuda.synchronize()

    # Measure
    timings = []
    for _ in range(7):
        if device == "cuda":
            torch.cuda.synchronize()
        t0 = time.perf_counter_ns()
        c = torch.mm(a, b)
        if device == "cuda":
            torch.cuda.synchronize()
        t1 = time.perf_counter_ns()
        timings.append(t1 - t0)

    return {
        "framework": "pytorch",
        "device": device,
        "size": size,
        "shape": f"{size}x{size}",
        "timings_ns": timings,
        "median_ns": sorted(timings)[len(timings) // 2],
        "checksum": float(c.sum().item()),
    }


if __name__ == "__main__":
    import json
    size = int(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_SIZE
    import torch
    results = []
    results.append(matmul_pytorch(size, "cpu"))
    if torch.cuda.is_available():
        results.append(matmul_pytorch(size, "cuda"))
    print(json.dumps(results, indent=2))
