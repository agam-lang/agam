fn butterfly_cost(size: i64) -> i64 {
    let mut total: i64 = 0;
    let mut stage: i64 = 1;
    while stage < size {
        let mut i: i64 = 0;
        while i < size {
            let left = ((i * 19) + stage) % 65521;
            let right = ((i * 23) + (stage * 3)) % 65521;
            total += (left + right) - (left - right);
            i += stage * 2;
        }
        stage *= 2;
    }
    total
}
fn main() { println!("{}", butterfly_cost(16384)); }
