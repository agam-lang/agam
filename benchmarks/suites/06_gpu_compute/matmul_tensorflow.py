"""GPU Matrix Multiplication — TensorFlow (CPU + GPU)."""
import sys
import time

DEFAULT_SIZE = 512


def matmul_tensorflow(size: int, device: str = "cpu") -> dict:
    import tensorflow as tf

    tf_device = "/GPU:0" if device == "gpu" else "/CPU:0"
    with tf.device(tf_device):
        a = tf.random.normal([size, size])
        b = tf.random.normal([size, size])

        # Warmup
        for _ in range(3):
            _ = tf.linalg.matmul(a, b)

        # Measure
        timings = []
        for _ in range(7):
            t0 = time.perf_counter_ns()
            c = tf.linalg.matmul(a, b)
            # Force completion
            _ = c.numpy()
            t1 = time.perf_counter_ns()
            timings.append(t1 - t0)

    return {
        "framework": "tensorflow",
        "device": device,
        "size": size,
        "shape": f"{size}x{size}",
        "timings_ns": timings,
        "median_ns": sorted(timings)[len(timings) // 2],
        "checksum": float(c.numpy().sum()),
    }


if __name__ == "__main__":
    import json
    import tensorflow as tf
    size = int(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_SIZE
    results = [matmul_tensorflow(size, "cpu")]
    if tf.config.list_physical_devices("GPU"):
        results.append(matmul_tensorflow(size, "gpu"))
    print(json.dumps(results, indent=2))
