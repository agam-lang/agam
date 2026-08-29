fn matrix_multiply(size: i64) -> i64 {
    let mut checksum: i64 = 0;
    let s = size;
    for i in 0..s {
        for j in 0..s {
            let mut sum: i64 = 0;
            for k in 0..s {
                let a = ((i * s + k) * 17 + 13) % 256;
                let b = ((k * s + j) * 19 + 7) % 256;
                sum += a * b;
            }
            checksum = (checksum * 31 + sum) % 1000000007;
        }
    }
    checksum
}

fn main() { println!("{}", matrix_multiply(64)); }
