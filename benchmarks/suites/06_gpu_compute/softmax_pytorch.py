"""GPU Softmax — PyTorch (CPU + CUDA)."""
import sys, time, json
DEFAULT_SIZE = 4096
DEFAULT_ROUNDS = 4000
def softmax_pytorch(width: int, rounds: int, device: str = "cpu") -> dict:
    import torch
    x = torch.randn(rounds, width, device=device)
    for _ in range(3): _ = torch.softmax(x, dim=1)
    if device == "cuda": torch.cuda.synchronize()
    timings = []
    for _ in range(7):
        if device == "cuda": torch.cuda.synchronize()
        t0 = time.perf_counter_ns()
        result = torch.softmax(x, dim=1)
        if device == "cuda": torch.cuda.synchronize()
        timings.append(time.perf_counter_ns() - t0)
    return {"framework": "pytorch", "device": device, "width": width, "rounds": rounds,
            "op": "softmax", "timings_ns": timings, "median_ns": sorted(timings)[3],
            "checksum": float(result.sum().item())}
if __name__ == "__main__":
    import torch
    w = int(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_SIZE
    r = int(sys.argv[2]) if len(sys.argv) > 2 else DEFAULT_ROUNDS
    results = [softmax_pytorch(w, r, "cpu")]
    if torch.cuda.is_available(): results.append(softmax_pytorch(w, r, "cuda"))
    print(json.dumps(results, indent=2))
