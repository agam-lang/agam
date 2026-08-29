"""GPU 2D Convolution — TensorFlow (CPU + GPU)."""
import sys, time, json
DEFAULT_SIZE = 512
def conv2d_tensorflow(size: int, device: str = "cpu") -> dict:
    import tensorflow as tf
    tf_device = "/GPU:0" if device == "gpu" else "/CPU:0"
    with tf.device(tf_device):
        x = tf.random.normal([1, size, size, 1])
        k = tf.random.normal([3, 3, 1, 1])
        for _ in range(3): _ = tf.nn.conv2d(x, k, strides=1, padding="VALID")
        timings = []
        for _ in range(7):
            t0 = time.perf_counter_ns()
            result = tf.nn.conv2d(x, k, strides=1, padding="VALID")
            _ = result.numpy()
            timings.append(time.perf_counter_ns() - t0)
    return {"framework": "tensorflow", "device": device, "size": size, "op": "conv2d",
            "timings_ns": timings, "median_ns": sorted(timings)[3],
            "checksum": float(result.numpy().sum())}
if __name__ == "__main__":
    import tensorflow as tf
    size = int(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_SIZE
    results = [conv2d_tensorflow(size, "cpu")]
    if tf.config.list_physical_devices("GPU"): results.append(conv2d_tensorflow(size, "gpu"))
    print(json.dumps(results, indent=2))
