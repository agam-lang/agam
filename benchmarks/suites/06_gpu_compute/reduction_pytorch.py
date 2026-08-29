"""GPU Reduction Sum — PyTorch (CPU + CUDA)."""
import sys, time, json
DEFAULT_SIZE = 10_000_000
def reduction_pytorch(size: int, device: str = "cpu") -> dict:
    import torch
    a = torch.randn(size, device=device)
    for _ in range(3): _ = torch.sum(a)
    if device == "cuda": torch.cuda.synchronize()
    timings = []
    for _ in range(7):
        if device == "cuda": torch.cuda.synchronize()
        t0 = time.perf_counter_ns()
        s = torch.sum(a)
        if device == "cuda": torch.cuda.synchronize()
        timings.append(time.perf_counter_ns() - t0)
    return {"framework": "pytorch", "device": device, "size": size,
            "timings_ns": timings, "median_ns": sorted(timings)[3],
            "checksum": float(s.item())}
if __name__ == "__main__":
    import torch
    size = int(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_SIZE
    results = [reduction_pytorch(size, "cpu")]
    if torch.cuda.is_available(): results.append(reduction_pytorch(size, "cuda"))
    print(json.dumps(results, indent=2))
