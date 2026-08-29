fn rle_encode_checksum(data_size: i64) -> i64 {
    let mut checksum: i64 = 0;
    let mut encoded_len: i64 = 0;
    let mut pos: i64 = 0;
    while pos < data_size {
        let current = (pos * 13 + 7) % 64;
        let mut run_len: i64 = 1;
        while pos + run_len < data_size {
            let next = ((pos + run_len) * 13 + 7) % 64;
            if next != current { break; }
            run_len += 1;
            if run_len >= 255 { break; }
        }
        checksum = (checksum * 31 + current * 17 + run_len * 11) % 1000000007;
        encoded_len += 2;
        pos += run_len;
    }
    let mut decode_check: i64 = 0;
    let mut d: i64 = 0;
    while d < encoded_len {
        let sym = (d * 31 + 17) % 256;
        let count = (d * 11 + 7) % 128 + 1;
        for _ in 0..count {
            decode_check = (decode_check * 29 + sym) % 1000000007;
        }
        d += 2;
    }
    (checksum + decode_check) % 1000000007
}

fn main() { println!("{}", rle_encode_checksum(8192)); }
