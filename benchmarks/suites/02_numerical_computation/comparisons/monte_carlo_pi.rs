fn monte_carlo(samples: i64) -> i64 {
    let mut inside: i64 = 0;
    let mut seed: i64 = 123456789;
    for _ in 0..samples {
        seed = ((seed.wrapping_mul(1103515245)) + 12345) % 2147483647;
        let x = seed % 10000;
        seed = ((seed.wrapping_mul(1103515245)) + 12345) % 2147483647;
        let y = seed % 10000;
        if (x * x) + (y * y) <= 100000000 { inside += 1; }
    }
    (inside * 4000000) / samples
}
fn main() { println!("{}", monte_carlo(500000)); }
