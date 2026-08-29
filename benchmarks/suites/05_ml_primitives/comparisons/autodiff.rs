fn autodiff_trace(steps: i64) -> i64 {
    let (mut value, mut grad) = (7i64, 1i64);
    for _ in 0..steps { grad += (value * 3) % 17; value += grad; }
    value + grad
}
fn main() { println!("{}", autodiff_trace(5000000)); }
