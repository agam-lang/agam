"""GPU Softmax — TensorFlow."""
import sys, time, json
DEFAULT_SIZE = 4096
DEFAULT_ROUNDS = 4000
def softmax_tf(width: int, rounds: int, device: str = "cpu") -> dict:
    import tensorflow as tf
    tf_device = "/GPU:0" if device == "gpu" else "/CPU:0"
    with tf.device(tf_device):
        x = tf.random.normal([rounds, width])
        for _ in range(3): _ = tf.nn.softmax(x, axis=1)
        timings = []
        for _ in range(7):
            t0 = time.perf_counter_ns()
            result = tf.nn.softmax(x, axis=1)
            _ = result.numpy()
            timings.append(time.perf_counter_ns() - t0)
    return {"framework": "tensorflow", "device": device, "width": width, "rounds": rounds,
            "op": "softmax", "timings_ns": timings, "median_ns": sorted(timings)[3],
            "checksum": float(result.numpy().sum())}
if __name__ == "__main__":
    import tensorflow as tf
    w = int(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_SIZE
    r = int(sys.argv[2]) if len(sys.argv) > 2 else DEFAULT_ROUNDS
    results = [softmax_tf(w, r, "cpu")]
    if tf.config.list_physical_devices("GPU"): results.append(softmax_tf(w, r, "gpu"))
    print(json.dumps(results, indent=2))
