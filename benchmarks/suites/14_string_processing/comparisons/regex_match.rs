fn regex_match(length: i64) -> i64 {
    let mut checksum: i64 = 0;
    for iter in 0..100 {
        let mut state: i64 = 0;
        let mut matches: i64 = 0;
        for i in 0..length {
            let c = ((i * 17 + iter * 13) % 5) + 97;
            if state == 0 {
                if c == 97 { state = 0; } else if c == 98 { state = 1; } else { state = 0; }
            } else if state == 1 {
                if c == 98 { state = 1; } else if c == 99 { state = 2; } else if c == 100 { state = 3; } else if c == 97 { state = 0; } else { state = 0; }
            } else if state == 2 {
                if c == 100 { state = 3; } else if c == 97 { state = 0; } else { state = 0; }
            } else if state == 3 {
                matches += 1;
                if c == 97 { state = 0; } else if c == 98 { state = 1; } else { state = 0; }
            }
        }
        checksum = (checksum * 31 + matches) % 1000000007;
    }
    checksum
}

fn main() { println!("{}", regex_match(10000)); }
