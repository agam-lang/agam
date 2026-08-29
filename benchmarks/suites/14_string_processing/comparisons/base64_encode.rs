fn base64_encode(length: i64) -> i64 {
    let mut checksum: i64 = 0;
    for iter in 0..100 {
        let mut i = 0;
        while i < length {
            let b1 = (i * 17 + iter * 13) % 256;
            let b2 = ((i+1) * 19 + iter * 7) % 256;
            let b3 = ((i+2) * 23 + iter * 11) % 256;
            
            let enc1 = (b1 >> 2) & 63;
            let enc2 = ((b1 & 3) << 4) | ((b2 >> 4) & 15);
            let enc3 = ((b2 & 15) << 2) | ((b3 >> 6) & 3);
            let enc4 = b3 & 63;
            
            checksum = (checksum * 31 + enc1) % 1000000007;
            checksum = (checksum * 31 + enc2) % 1000000007;
            checksum = (checksum * 31 + enc3) % 1000000007;
            checksum = (checksum * 31 + enc4) % 1000000007;
            
            i += 3;
        }
    }
    checksum
}

fn main() { println!("{}", base64_encode(10000)); }
