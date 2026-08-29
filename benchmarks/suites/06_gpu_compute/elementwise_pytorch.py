"""GPU Elementwise FMA — PyTorch (CPU + CUDA)."""
import sys, time, json
DEFAULT_SIZE = 10_000_000
def elementwise_pytorch(size: int, device: str = "cpu") -> dict:
    import torch
    a = torch.randn(size, device=device)
    b = torch.randn(size, device=device)
    c = torch.randn(size, device=device)
    for _ in range(3): _ = torch.addcmul(c, a, b, value=1.0)
    if device == "cuda": torch.cuda.synchronize()
    timings = []
    for _ in range(7):
        if device == "cuda": torch.cuda.synchronize()
        t0 = time.perf_counter_ns()
        result = torch.addcmul(c, a, b, value=1.0)
        if device == "cuda": torch.cuda.synchronize()
        timings.append(time.perf_counter_ns() - t0)
    return {"framework": "pytorch", "device": device, "size": size, "op": "fma",
            "timings_ns": timings, "median_ns": sorted(timings)[3],
            "checksum": float(result.sum().item())}
if __name__ == "__main__":
    import torch
    size = int(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_SIZE
    results = [elementwise_pytorch(size, "cpu")]
    if torch.cuda.is_available(): results.append(elementwise_pytorch(size, "cuda"))
    print(json.dumps(results, indent=2))
