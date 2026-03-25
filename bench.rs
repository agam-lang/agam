use std::time::Instant;

fn fib(n: u64) -> u64 {
    if n <= 1 {
        return n;
    }
    fib(n - 1) + fib(n - 2)
}

fn main() {
    println!("Starting Native (Agam Backend) Benchmark...");
    let start_time = Instant::now();
    let result = fib(38);
    let duration = start_time.elapsed();

    println!("Result: {}", result);
    println!("Time taken (Native): {:.4} seconds", duration.as_secs_f64());
}
