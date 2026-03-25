import time

def fib(n):
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)

print("Starting Python Benchmark...")
start_time = time.time()
result = fib(38)
end_time = time.time()

print(f"Result: {result}")
print(f"Time taken (Python): {end_time - start_time:.4f} seconds")
