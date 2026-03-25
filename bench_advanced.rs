/// Agam Advanced Benchmark Suite — Rust Native Harness
/// Equivalent to bench_advanced.agam / bench_advanced.py
/// Uses the same algorithms to measure Agam's native Rust-compiled performance.
///
/// Run: cargo run --release --bin bench_advanced

fn sum_loop(n: i64) -> i64 {
    let mut total: i64 = 0;
    let mut i: i64 = 0;
    while i < n {
        total += i;
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
    for i in 0..steps {
        sum += i * i;
    }
    sum
}

fn run_bench(name: &str, f: fn() -> i64) -> f64 {
    let start = std::time::Instant::now();
    let result = f();
    let elapsed = start.elapsed().as_secs_f64();
    println!("  {}: {:.4}s  (result={})", name, elapsed, result);
    elapsed
}

fn main() {
    println!("{}", "=".repeat(65));
    println!("  Agam vs Python — Advanced Benchmark Suite");
    println!("  Agam Runtime (Rust native, --release)");
    println!("{}", "=".repeat(65));
    println!();

    let mut total = 0.0;

    total += run_bench("Sum(100M)", || sum_loop(100_000_000));
    total += run_bench("Fibonacci(40)", || fibonacci(40));
    total += run_bench("PrimeCount(100K)", || count_primes(100_000));
    total += run_bench("MatMul(100x100)", || matrix_multiply(100));
    total += run_bench("Integrate(10M)", || integrate_x2(10_000_000));

    println!();
    println!("  Total Agam time: {:.4}s", total);
    println!("{}", "=".repeat(65));
}
