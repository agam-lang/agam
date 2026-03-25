"""
Agam vs Python — Advanced Benchmark Suite
═══════════════════════════════════════════
Equivalent Python implementations for direct speed comparison.
Run: python bench_advanced.py
"""
import time


def sum_loop(n: int) -> int:
    """Benchmark 1: Sum of first N integers using a loop."""
    total = 0
    for i in range(n):
        total += i
    return total


def fibonacci(n: int) -> int:
    """Benchmark 2: Fibonacci (iterative) — compute Fib(N)."""
    a, b = 0, 1
    for _ in range(n):
        a, b = b, a + b
    return a


def count_primes(n: int) -> int:
    """Benchmark 3: Prime sieve — count primes below N."""
    count = 0
    for num in range(2, n):
        is_prime = True
        d = 2
        while d * d <= num:
            if num % d == 0:
                is_prime = False
                break
            d += 1
        if is_prime:
            count += 1
    return count


def matrix_multiply(size: int) -> int:
    """Benchmark 4: Matrix multiply (flat) — NxN * NxN."""
    total = 0
    for i in range(size):
        for j in range(size):
            val = 0
            for k in range(size):
                val += (i * size + k) * (k * size + j)
            total += val
    return total


def integrate_x2(steps: int) -> int:
    """Benchmark 5: Numerical integration (sum of i^2)."""
    total = 0
    for i in range(steps):
        total += i * i
    return total


def run_benchmark(name: str, func, *args):
    """Run a benchmark and return the elapsed time."""
    start = time.perf_counter()
    result = func(*args)
    elapsed = time.perf_counter() - start
    print(f"  {name}: {elapsed:.4f}s  (result={result})")
    return elapsed


def main():
    print("=" * 65)
    print("  Agam vs Python — Advanced Benchmark Suite")
    print("  Python " + f"{__import__('sys').version.split()[0]}")
    print("=" * 65)
    print()

    total = 0.0

    # Benchmark 1: Sum loop (100 million iterations)
    t = run_benchmark("Sum(100M)", sum_loop, 100_000_000)
    total += t

    # Benchmark 2: Fibonacci(40)
    t = run_benchmark("Fibonacci(40)", fibonacci, 40)
    total += t

    # Benchmark 3: Prime count below 100,000
    t = run_benchmark("PrimeCount(100K)", count_primes, 100_000)
    total += t

    # Benchmark 4: Matrix multiply 100x100
    t = run_benchmark("MatMul(100x100)", matrix_multiply, 100)
    total += t

    # Benchmark 5: Numerical integration
    t = run_benchmark("Integrate(10M)", integrate_x2, 10_000_000)
    total += t

    print()
    print(f"  Total Python time: {total:.4f}s")
    print("=" * 65)


if __name__ == "__main__":
    main()
