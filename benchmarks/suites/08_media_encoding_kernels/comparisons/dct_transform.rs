fn dct_checksum(blocks: i64) -> i64 {
    let mut checksum: i64 = 0;
    for b in 0..blocks {
        let mut input_sum: i64 = 0;
        let mut output_sum: i64 = 0;
        for u in 0..8 {
            for v in 0..8 {
                let mut sum: i64 = 0;
                for x in 0..8 {
                    for y in 0..8 {
                        let pixel = ((b * 64 + x * 8 + y) * 17 + 13) % 256;
                        if u == 0 && v == 0 { input_sum = (input_sum + pixel) % 1000000007; }
                        let cos_x = ((2 * x + 1) * u * 314159) / 1600000;
                        let cos_y = ((2 * y + 1) * v * 314159) / 1600000;
                        let cx = (1000 * (100 - (cos_x * cos_x) / 20000)) / 100;
                        let cy = (1000 * (100 - (cos_y * cos_y) / 20000)) / 100;
                        sum += (pixel * cx * cy) / 1000000;
                    }
                }
                let cu = if u == 0 { 707 } else { 1000 };
                let cv = if v == 0 { 707 } else { 1000 };
                let dct_val = (sum * cu * cv) / 4000000;
                output_sum = (output_sum * 31 + (dct_val + 10000)) % 1000000007;
            }
        }
        checksum = (checksum * 37 + input_sum + output_sum) % 1000000007;
    }
    checksum
}

fn main() { println!("{}", dct_checksum(64)); }
