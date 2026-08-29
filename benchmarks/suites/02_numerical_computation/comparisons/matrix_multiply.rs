fn matrix_checksum(size: i64) -> i64 {
    let mut sum: i64 = 0;
    for row in 0..size {
        for col in 0..size {
            let mut cell: i64 = 0;
            for inner in 0..size {
                cell += ((row * inner) + 3) * ((inner * col) + 5);
            }
            sum += cell % 104729;
        }
    }
    sum
}
fn main() { println!("{}", matrix_checksum(64)); }
