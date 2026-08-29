fn lz77_compress_checksum(data_size: i64, window_size: i64) -> i64 {
    let mut checksum: i64 = 0;
    let mut pos: i64 = 0;
    while pos < data_size {
        let mut best_len: i64 = 0;
        let mut best_dist: i64 = 0;
        let search_start = if pos - window_size < 0 { 0 } else { pos - window_size };
        for s in search_start..pos {
            let mut match_len: i64 = 0;
            while pos + match_len < data_size {
                let a = (s + match_len) * 7 + 13;
                let b = (pos + match_len) * 7 + 13;
                if a % 256 != b % 256 { break; }
                match_len += 1;
                if match_len > 15 { break; }
            }
            if match_len > best_len { best_len = match_len; best_dist = pos - s; }
        }
        if best_len >= 3 {
            checksum = (checksum * 31 + best_dist * 17 + best_len * 13) % 1000000007;
            pos += best_len;
        } else {
            let literal = pos * 7 + 13;
            checksum = (checksum * 31 + literal % 256) % 1000000007;
            pos += 1;
        }
    }
    checksum
}

fn main() { println!("{}", lz77_compress_checksum(512, 32)); }
