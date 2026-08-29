fn audio_lpc_checksum(frames: i64) -> i64 {
    let order: i64 = 10;
    let frame_size: i64 = 256;
    let mut checksum: i64 = 0;
    
    for f in 0..frames {
        let mut max_r: i64 = 0;
        for lag in 0..=order {
            let mut r: i64 = 0;
            for i in 0..(frame_size - lag) {
                let s1 = ((f * frame_size + i) * 17 + 13) % 65536 - 32768;
                let s2 = ((f * frame_size + i + lag) * 17 + 13) % 65536 - 32768;
                r += (s1 * s2) / 32768;
            }
            if r < 0 { r = -r; }
            if r > max_r { max_r = r; }
            checksum = (checksum * 31 + r) % 1000000007;
        }
        
        let mut residual: i64 = 0;
        for i_r in order..frame_size {
            let mut pred: i64 = 0;
            for j in 1..=order {
                let s = ((f * frame_size + i_r - j) * 17 + 13) % 65536 - 32768;
                let coef = (j * 17) % 100;
                pred += (s * coef) / 100;
            }
            let actual = ((f * frame_size + i_r) * 17 + 13) % 65536 - 32768;
            let mut diff = actual - pred;
            if diff < 0 { diff = -diff; }
            residual += diff;
        }
        checksum = (checksum * 37 + residual + max_r) % 1000000007;
    }
    checksum
}

fn main() { println!("{}", audio_lpc_checksum(32)); }
