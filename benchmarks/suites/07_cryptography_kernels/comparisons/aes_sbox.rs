fn aes_sbox_val(input: i64) -> i64 {
    let x = input & 255;
    let mut inv: i64 = 1;
    for _ in 0..7 {
        inv = (inv * x) % 257;
        if inv > 255 { inv ^= 283; }
    }
    inv &= 255;
    let result = inv ^ ((inv << 1) & 255) ^ ((inv << 2) & 255) ^
                 ((inv << 3) & 255) ^ ((inv << 4) & 255) ^ 99;
    result & 255
}

fn aes_checksum(blocks: i64) -> i64 {
    let mut checksum: i64 = 0;
    for b in 0..blocks {
        let mut state: i64 = 0;
        for byte_idx in 0..16 {
            let input_byte = (b * 16 + byte_idx) * 1103515245 + 12345;
            let sub = aes_sbox_val(input_byte & 255);
            let shifted = sub ^ ((byte_idx * 3) & 255);
            let mut mixed = shifted * 2;
            if mixed > 255 { mixed ^= 283; }
            state = (state * 31 + mixed) % 1000000007;
        }
        checksum = (checksum * 37 + state) % 1000000007;
    }
    checksum
}

fn main() { println!("{}", aes_checksum(512)); }
