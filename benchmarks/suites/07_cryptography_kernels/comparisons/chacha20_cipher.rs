fn chacha_quarter(a: i64, b: i64, c: i64, d: i64) -> i64 {
    let mask: i64 = 4294967295;
    let av = (a + b) & mask;
    let dv = ((d ^ av) << 16 | ((d ^ av) & mask) >> 16) & mask;
    let cv = (c + dv) & mask;
    let bv = ((b ^ cv) << 12 | ((b ^ cv) & mask) >> 20) & mask;
    let av2 = (av + bv) & mask;
    let dv2 = ((dv ^ av2) << 8 | ((dv ^ av2) & mask) >> 24) & mask;
    let cv2 = (cv + dv2) & mask;
    let bv2 = ((bv ^ cv2) << 7 | ((bv ^ cv2) & mask) >> 25) & mask;
    (av2 + bv2 + cv2 + dv2) & mask
}

fn chacha20_checksum(rounds: i64) -> i64 {
    let mut checksum: i64 = 0;
    for i in 0..rounds {
        let mut s0: i64 = 1634760805;
        let mut s1: i64 = 857760878;
        let mut s2: i64 = 2036477234;
        let mut s3: i64 = 1797285236;
        for r in 0..10 {
            let q1 = chacha_quarter(s0, s1, s2, s3);
            let q2 = chacha_quarter(s1 + i, s2 + r, s3, s0);
            let q3 = chacha_quarter(s2, s3 + i, s0 + r, s1);
            let q4 = chacha_quarter(s3, s0, s1 + i, s2 + r);
            s0 = q1; s1 = q2; s2 = q3; s3 = q4;
        }
        checksum = (checksum * 37 + s0 + s1 + s2 + s3) % 1000000007;
    }
    checksum
}

fn main() { println!("{}", chacha20_checksum(1024)); }
