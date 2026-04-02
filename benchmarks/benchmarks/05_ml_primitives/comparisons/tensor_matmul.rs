fn matmul_score(size: i64) -> i64 {
    let mut total = 0;
    for row in 0..size {
        for col in 0..size {
            let mut cell = 0;
            for inner in 0..size {
                cell += ((row + inner) % 31) * ((inner + col) % 29);
            }
            total += cell;
        }
    }
    total
}

fn main() {
    println!("{}", matmul_score(48));
}

