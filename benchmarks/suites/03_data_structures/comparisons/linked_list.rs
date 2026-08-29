fn pointer_chase(nodes: i64, rounds: i64) -> i64 {
    let mut cursor: i64 = 1;
    let mut checksum: i64 = 0;
    for _ in 0..rounds {
        cursor = ((cursor.wrapping_mul(1103515245)) + 12345) % nodes;
        checksum += cursor;
    }
    checksum
}
fn main() { println!("{}", pointer_chase(1000003, 8000000)); }
