"""GPU 2D Convolution — PyTorch (CPU + CUDA)."""
import sys, time, json
DEFAULT_SIZE = 512
def conv2d_pytorch(size: int, device: str = "cpu") -> dict:
    import torch
    import torch.nn.functional as F
    x = torch.randn(1, 1, size, size, device=device)
    k = torch.randn(1, 1, 3, 3, device=device)
    for _ in range(3): _ = F.conv2d(x, k)
    if device == "cuda": torch.cuda.synchronize()
    timings = []
    for _ in range(7):
        if device == "cuda": torch.cuda.synchronize()
        t0 = time.perf_counter_ns()
        result = F.conv2d(x, k)
        if device == "cuda": torch.cuda.synchronize()
        timings.append(time.perf_counter_ns() - t0)
    return {"framework": "pytorch", "device": device, "size": size, "op": "conv2d",
            "timings_ns": timings, "median_ns": sorted(timings)[3],
            "checksum": float(result.sum().item())}
if __name__ == "__main__":
    import torch
    size = int(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_SIZE
    results = [conv2d_pytorch(size, "cpu")]
    if torch.cuda.is_available(): results.append(conv2d_pytorch(size, "cuda"))
    print(json.dumps(results, indent=2))
