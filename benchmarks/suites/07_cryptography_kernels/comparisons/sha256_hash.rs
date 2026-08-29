fn rotr32(x: i64, n: i64) -> i64 {
    let mask: i64 = 4294967295;
    let xm = x & mask;
    ((xm >> n) | (xm << (32 - n))) & mask
}

fn sha256_checksum(blocks: i64) -> i64 {
    let h0: i64 = 1779033703;
    let h1: i64 = 3144134277;
    let h2: i64 = 1013904242;
    let h3: i64 = 2773480762;
    let h4: i64 = 1359893119;
    let h5: i64 = 2600822924;
    let h6: i64 = 528734635;
    let h7: i64 = 1541459225;
    let mask: i64 = 4294967295;
    let mut checksum: i64 = 0;
    
    for b in 0..blocks {
        let mut a = h0; let mut bv = h1; let mut c = h2; let mut d = h3;
        let mut e = h4; let mut f = h5; let mut g = h6; let mut hv = h7;
        
        for round in 0..64 {
            let w = (b * 64 + round) * 1103515245 + 12345;
            let s1 = rotr32(e, 6) ^ rotr32(e, 11) ^ rotr32(e, 25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = (hv + s1 + (ch & mask) + (w & mask) + round * 7) & mask;
            let s0 = rotr32(a, 2) ^ rotr32(a, 13) ^ rotr32(a, 22);
            let maj = (a & bv) ^ (a & c) ^ (bv & c);
            let temp2 = (s0 + (maj & mask)) & mask;
            
            hv = g; g = f; f = e; e = (d + temp1) & mask;
            d = c; c = bv; bv = a; a = (temp1 + temp2) & mask;
        }
        checksum = (checksum * 31 + a + bv + e + hv) % 1000000007;
    }
    checksum
}

fn main() { println!("{}", sha256_checksum(256)); }
