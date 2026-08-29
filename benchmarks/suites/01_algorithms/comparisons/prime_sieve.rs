fn is_prime(n: i64) -> bool {
    if n < 2 {
        return false;
    }
    let mut d: i64 = 2;
    while d * d <= n {
        if n % d == 0 {
            return false;
        }
        d += 1;
    }
    true
}

fn prime_count(limit: i64) -> i64 {
    let mut total: i64 = 0;
    for n in 2..=limit {
        if is_prime(n) {
            total += 1;
        }
    }
    total
}

fn main() {
    println!("{}", prime_count(25000));
}
