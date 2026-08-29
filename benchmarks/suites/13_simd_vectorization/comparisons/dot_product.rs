fn dot_product(size: i64) -> i64 {
    let mut sum: i64 = 0;
    for i in 0..size {
        let a = ((i * 17 + 13) % 256) - 128;
        let b = ((i * 19 + 7) % 256) - 128;
        sum += a * b;
    }
    sum
}

fn main() { println!("{}", dot_product(65536)); }
