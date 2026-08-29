fn json_parse(length: i64) -> i64 {
    let mut checksum: i64 = 0;
    for iter in 0..100 {
        let mut state: i64 = 0;
        let mut elements: i64 = 0;
        let mut depth: i64 = 0;
        for i in 0..length {
            let mut c = (i * 19 + iter * 7) % 6;
            if c == 0 { c = 123; } else if c == 1 { c = 125; } else if c == 2 { c = 34; } else if c == 3 { c = 58; } else if c == 4 { c = 44; } else { c = 97; }
            
            if state == 0 { if c == 123 { state = 1; depth += 1; } }
            else if state == 1 { if c == 34 { state = 2; } else if c == 125 { depth -= 1; if depth == 0 { state = 0; } else { state = 6; } } }
            else if state == 2 { if c == 34 { state = 3; } }
            else if state == 3 { if c == 58 { state = 4; } }
            else if state == 4 { if c == 34 { state = 5; } else if c == 123 { state = 1; depth += 1; } }
            else if state == 5 { if c == 34 { state = 6; elements += 1; } }
            else if state == 6 { if c == 44 { state = 1; } else if c == 125 { depth -= 1; if depth == 0 { state = 0; } } }
        }
        checksum = (checksum * 31 + elements + depth) % 1000000007;
    }
    checksum
}

fn main() { println!("{}", json_parse(10000)); }
