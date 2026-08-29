"""GPU Reduction Sum — TensorFlow."""
import sys, time, json
DEFAULT_SIZE = 10_000_000
def reduction_tf(size: int, device: str = "cpu") -> dict:
    import tensorflow as tf
    tf_device = "/GPU:0" if device == "gpu" else "/CPU:0"
    with tf.device(tf_device):
        a = tf.random.normal([size])
        for _ in range(3): _ = tf.reduce_sum(a)
        timings = []
        for _ in range(7):
            t0 = time.perf_counter_ns()
            s = tf.reduce_sum(a)
            _ = s.numpy()
            timings.append(time.perf_counter_ns() - t0)
    return {"framework": "tensorflow", "device": device, "size": size,
            "timings_ns": timings, "median_ns": sorted(timings)[3],
            "checksum": float(s.numpy())}
if __name__ == "__main__":
    import tensorflow as tf
    size = int(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_SIZE
    results = [reduction_tf(size, "cpu")]
    if tf.config.list_physical_devices("GPU"): results.append(reduction_tf(size, "gpu"))
    print(json.dumps(results, indent=2))
