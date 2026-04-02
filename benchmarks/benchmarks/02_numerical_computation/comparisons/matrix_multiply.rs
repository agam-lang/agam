fn matrix_checksum(size: i64) -> i64 {
    let mut total = 0;
    for row in 0..size {
        for col in 0..size {
            let mut cell = 0;
            for inner in 0..size {
                cell += ((row * inner) + 3) * ((inner * col) + 5);
            }
            total += cell % 104_729;
        }
    }
    total
}

fn main() {
    println!("{}", matrix_checksum(64));
}

