fn crc32_checksum(data_size: i64) -> i64 {
    let mut crc: i64 = 4294967295;
    let mask: i64 = 4294967295;
    let poly: i64 = 3988292384;
    for i in 0..data_size {
        let byte_val = (i * 1103515245 + 12345) & 255;
        crc ^= byte_val;
        for _ in 0..8 {
            if (crc & 1) != 0 {
                crc = ((crc >> 1) & (mask >> 1)) ^ poly;
            } else {
                crc = (crc >> 1) & (mask >> 1);
            }
        }
    }
    (crc ^ mask) % 1000000007
}

fn main() { println!("{}", crc32_checksum(16384)); }
