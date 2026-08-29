fn block_sort_checksum(size: i64) -> i64 {
    let mut checksum: i64 = 0;
    for i in 0..size {
        let mut rot_checksum: i64 = 0;
        for r in 0..size {
            let idx = (i + r) % size;
            let val = (idx * 7 + 13) % 256;
            rot_checksum = (rot_checksum * 31 + val) % 1000000007;
        }
        checksum = (checksum + rot_checksum) % 1000000007;
    }
    let mut sorted_check: i64 = 0;
    for j in 0..size {
        let last_col = ((j + size - 1) % size * 7 + 13) % 256;
        sorted_check = (sorted_check * 37 + last_col * (j + 1)) % 1000000007;
    }
    (checksum + sorted_check) % 1000000007
}

fn main() { println!("{}", block_sort_checksum(256)); }
