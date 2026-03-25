import time
import numpy as np
import pandas as pd

def optimize_rosenbrock():
    print("Python: Optimizing Rosenbrock using manual Adam (50,000 iters)...")
    def grad_f(x):
        return np.array([
            -2.0 * (1.0 - x[0]) - 400.0 * x[0] * (x[1] - x[0]**2),
            200.0 * (x[1] - x[0]**2)
        ])
    
    x = np.array([-1.0, 2.0])
    m = np.zeros(2)
    v = np.zeros(2)
    beta1, beta2, epsilon = 0.9, 0.999, 1e-8
    lr = 0.001
    
    for t in range(1, 50001):
        g = grad_f(x)
        if np.linalg.norm(g) < 1e-8:
            break
        m = beta1 * m + (1.0 - beta1) * g
        v = beta2 * v + (1.0 - beta2) * g**2
        m_hat = m / (1.0 - beta1**t)
        v_hat = v / (1.0 - beta2**t)
        x -= lr * m_hat / (np.sqrt(v_hat) + epsilon)
        
    print("Found minimum:", x)

def process_dataframe():
    print("Python (Pandas): Processing DataFrame (30 million rows)...")
    rows = 30_000_000
    
    # We use numpy optimized vectorization since pure python looping would take hours
    scores = np.sin(np.arange(rows) * 0.1)
    
    # Pandas overhead
    df = pd.DataFrame({
        "id": np.arange(rows),
        "score": scores
    })
    
    filtered = df[df["score"] > 0.5]
    mean_score = filtered["score"].mean()
    print("Mean score of filtered:", mean_score)

if __name__ == "__main__":
    start = time.time()
    optimize_rosenbrock()
    process_dataframe()
    end = time.time()
    print(f"Time taken (Python w/ Pandas/Numpy C-bindings): {end - start:.4f} seconds")
