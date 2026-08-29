fn paeth_predictor(left: i64, top: i64, top_left: i64) -> i64 {
    let p = left + top - top_left;
    let pa = (p - left).abs();
    let pb = (p - top).abs();
    let pc = (p - top_left).abs();

    if pa <= pb && pa <= pc {
        left
    } else if pb <= pc {
        top
    } else {
        top_left
    }
}

fn webp_encode(width: i64, height: i64) -> i64 {
    let mod_byte: i64 = 256;
    let mod_prime: i64 = 1000000007;

    let mut residual_sum: i64 = 0;
    for y in 1..height {
        for x in 1..width {
            let left = (((x - 1) * 17) + (y * 23)) % mod_byte;
            let top = ((x * 17) + ((y - 1) * 23)) % mod_byte;
            let top_left = (((x - 1) * 17) + ((y - 1) * 23)) % mod_byte;
            let current = ((x * 17) + (y * 23)) % mod_byte;

            let predicted = paeth_predictor(left, top, top_left);
            let diff = (current - predicted).abs();
            residual_sum = (residual_sum + diff) % mod_prime;
        }
    }
    residual_sum
}

fn main() {
    let res = webp_encode(512, 512);
    println!("{}", res);
}
