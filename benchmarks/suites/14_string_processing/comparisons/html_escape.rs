fn html_escape(length: i64) -> i64 {
    let mut checksum: i64 = 0;
    for iter in 0..100 {
        let mut out_len: i64 = 0;
        for i in 0..length {
            let c = (i * 23 + iter * 11) % 10;
            if c == 0 { out_len += 4; checksum = (checksum * 31 + 60) % 1000000007; }
            else if c == 1 { out_len += 4; checksum = (checksum * 31 + 62) % 1000000007; }
            else if c == 2 { out_len += 5; checksum = (checksum * 31 + 38) % 1000000007; }
            else if c == 3 { out_len += 6; checksum = (checksum * 31 + 34) % 1000000007; }
            else if c == 4 { out_len += 5; checksum = (checksum * 31 + 39) % 1000000007; }
            else { out_len += 1; checksum = (checksum * 31 + 97 + c) % 1000000007; }
        }
        checksum = (checksum * 37 + out_len) % 1000000007;
    }
    checksum
}

fn main() { println!("{}", html_escape(10000)); }
