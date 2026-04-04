fn token_frequency_cost(tokens: i64, buckets: i64) -> i64 {
    let mut total = 0_i64;
    for token in 0..tokens {
        let bucket = ((token * 19) + (token / 7)) % buckets;
        if bucket < 8 {
            total += (bucket * 7) + 3;
        } else if bucket < 24 {
            total += (bucket * 3) + 1;
        } else {
            total += bucket + 11;
        }
    }
    total
}

fn main() {
    println!("{}", token_frequency_cost(7000000, 64));
}
