fn softmax_like(width: i64, rounds: i64) -> i64 {
    let mut total: i64 = 0;
    for r in 0..rounds {
        let mut partial: i64 = 0;
        for l in 0..width { partial += ((l * 11) + r) % 251; }
        total += partial;
    }
    total
}
fn main() { println!("{}", softmax_like(4096, 4000)); }
