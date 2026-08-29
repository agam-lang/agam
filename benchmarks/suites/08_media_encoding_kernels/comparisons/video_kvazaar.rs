fn kvazaar_intra_predict(block_size: i64) -> i64 {
    let mod_byte: i64 = 256;
    let mod_prime: i64 = 1000000007;

    let mut checksum: i64 = 0;
    for mode in 0..35i64 {
        let pred_val = mode * 7;
        for y in 0..block_size {
            for x in 0..block_size {
                let pixel = ((x * 13) + (y * 17) + pred_val) % mod_byte;
                checksum = ((checksum * 31) + pixel) % mod_prime;
            }
        }
    }
    checksum
}

fn main() {
    let res = kvazaar_intra_predict(32);
    println!("{}", res);
}
