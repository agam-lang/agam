/// Agam Advanced Benchmark Suite — Rust Native Harness
/// Equivalent to bench_advanced.agam / bench_advanced.py
/// Uses the same algorithms to measure Agam's native Rust-compiled performance.
///
/// Run: cargo run --release --bin bench_advanced

use std::env;

fn sum_loop(n: i64) -> i64 {
    let mut total: i64 = 0;
    let mut state: i64 = (n % 7_919) + 1;
    let mut i: i64 = 0;
    while i < n {
        state = (state * 57 + i * 13 + 17) % 1_000_003;
        total += state % 1_024;
        i += 1;
    }
    total
}

fn fibonacci(n: i64) -> i64 {
    let mut a: i64 = 0;
    let mut b: i64 = 1;
    for _ in 0..n {
        let temp = a + b;
        a = b;
        b = temp;
    }
    a
}

fn count_primes(n: i64) -> i64 {
    let mut count: i64 = 0;
    for num in 2..n {
        let mut is_prime = true;
        let mut d = 2i64;
        while d * d <= num {
            if num % d == 0 {
                is_prime = false;
                break;
            }
            d += 1;
        }
        if is_prime {
            count += 1;
        }
    }
    count
}

fn matrix_multiply(size: i64) -> i64 {
    let mut sum: i64 = 0;
    for i in 0..size {
        for j in 0..size {
            let mut val: i64 = 0;
            for k in 0..size {
                val += (i * size + k) * (k * size + j);
            }
            sum += val;
        }
    }
    sum
}

fn integrate_x2(steps: i64) -> i64 {
    let mut sum: i64 = 0;
    let mut wobble: i64 = (steps % 1_237) + 3;
    for i in 0..steps {
        wobble = (wobble * 73 + 19) % 65_521;
        sum += ((i * i) + wobble) % 4_096;
    }
    sum
}

fn run_bench<F>(name: &str, f: F) -> f64
where
    F: FnOnce() -> i64,
{
    let start = std::time::Instant::now();
    let result = f();
    let elapsed = start.elapsed().as_secs_f64();
    println!("  {}: {:.4}s  (result={})", name, elapsed, result);
    elapsed
}

fn main() {
    let mut values = [100_000_000_i64, 40, 100_000, 100, 10_000_000];
    for (slot, raw) in values.iter_mut().zip(env::args().skip(1)) {
        *slot = raw.parse().expect("expected integer benchmark argument");
    }
    let [sum_n, fib_n, prime_n, mat_n, integrate_n] = values;

    println!("{}", "=".repeat(65));
    println!("  Agam vs Python — Advanced Benchmark Suite");
    println!("  Agam Runtime (Rust native, --release)");
    println!("{}", "=".repeat(65));
    println!();

    let mut total = 0.0;

    total += run_bench(&format!("Sum({sum_n})"), || sum_loop(sum_n));
    total += run_bench(&format!("Fibonacci({fib_n})"), || fibonacci(fib_n));
    total += run_bench(&format!("PrimeCount({prime_n})"), || count_primes(prime_n));
    total += run_bench(&format!("MatMul({mat_n}x{mat_n})"), || matrix_multiply(mat_n));
    total += run_bench(&format!("Integrate({integrate_n})"), || integrate_x2(integrate_n));

    println!();
    println!("  Total Agam time: {:.4}s", total);
    println!("{}", "=".repeat(65));
}
