"""
Agam vs Python — Advanced Benchmark Suite
═══════════════════════════════════════════
Equivalent Python implementations for direct speed comparison.
Run: python bench_advanced.py
"""
import time
import sys


def sum_loop(n: int) -> int:
    """Benchmark 1: Stateful integer accumulation."""
    total = 0
    state = (n % 7919) + 1
    for i in range(n):
        state = (state * 57 + i * 13 + 17) % 1_000_003
        total += state % 1024
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
    """Benchmark 5: Polynomial accumulation with runtime wobble."""
    total = 0
    wobble = (steps % 1237) + 3
    for i in range(steps):
        wobble = (wobble * 73 + 19) % 65_521
        total += ((i * i) + wobble) % 4096
    return total


def run_benchmark(name: str, func, *args):
    """Run a benchmark and return the elapsed time."""
    start = time.perf_counter()
    result = func(*args)
    elapsed = time.perf_counter() - start
    print(f"  {name}: {elapsed:.4f}s  (result={result})")
    return elapsed


def main():
    defaults = [100_000_000, 40, 100_000, 100, 10_000_000]
    values = defaults[:]
    for i, raw in enumerate(sys.argv[1:6]):
        values[i] = int(raw)
    sum_n, fib_n, prime_n, mat_n, integrate_n = values

    print("=" * 65)
    print("  Agam vs Python — Advanced Benchmark Suite")
    print("  Python " + f"{__import__('sys').version.split()[0]}")
    print("=" * 65)
    print()

    total = 0.0

    t = run_benchmark(f"Sum({sum_n})", sum_loop, sum_n)
    total += t

    t = run_benchmark(f"Fibonacci({fib_n})", fibonacci, fib_n)
    total += t

    t = run_benchmark(f"PrimeCount({prime_n})", count_primes, prime_n)
    total += t

    t = run_benchmark(f"MatMul({mat_n}x{mat_n})", matrix_multiply, mat_n)
    total += t

    t = run_benchmark(f"Integrate({integrate_n})", integrate_x2, integrate_n)
    total += t

    print()
    print(f"  Total Python time: {total:.4f}s")
    print("=" * 65)


if __name__ == "__main__":
    main()
