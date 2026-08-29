fn huffman_checksum(data_size: i64) -> i64 {
    let mut checksum: i64 = 0;
    let mut total_bits: i64 = 0;
    for j in 0..data_size {
        let byte_val = (j * 7 + 13) % 256;
        let mut depth: i64 = 1;
        let mut val = byte_val;
        while val > 1 { val /= 2; depth += 1; }
        let mut code_len = depth;
        if code_len < 2 { code_len = 2; }
        if code_len > 12 { code_len = 12; }
        total_bits += code_len;
        let code = byte_val % (1_i64 << code_len);
        checksum = (checksum * 37 + code * 19 + code_len * 7) % 1000000007;
    }
    (checksum * 53 + total_bits) % 1000000007
}

fn main() { println!("{}", huffman_checksum(4096)); }
