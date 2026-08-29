fn abs_diff(a: i64, b: i64) -> i64 { if a > b { a - b } else { b - a } }

fn motion_est_checksum(frames: i64) -> i64 {
    let width: i64 = 64; let height: i64 = 64;
    let block_size: i64 = 8; let search_range: i64 = 4;
    let mut checksum: i64 = 0;
    
    for f in 0..frames {
        for by in 0..(height / block_size) {
            for bx in 0..(width / block_size) {
                let mut best_sad: i64 = 999999999;
                let mut best_dy: i64 = 0;
                let mut best_dx: i64 = 0;
                
                for dy in -search_range..=search_range {
                    for dx in -search_range..=search_range {
                        let mut sad: i64 = 0;
                        for r in 0..block_size {
                            for c in 0..block_size {
                                let cy = by * block_size + r;
                                let cx = bx * block_size + c;
                                let ry = cy + dy; let rx = cx + dx;
                                
                                let cur_pixel = ((f * width * height + cy * width + cx) * 17 + 13) % 256;
                                let mut ref_pixel: i64 = 0;
                                if ry >= 0 && ry < height && rx >= 0 && rx < width {
                                    ref_pixel = (((f - 1) * width * height + ry * width + rx) * 17 + 13) % 256;
                                }
                                sad += abs_diff(cur_pixel, ref_pixel);
                            }
                        }
                        if sad < best_sad { best_sad = sad; best_dy = dy; best_dx = dx; }
                    }
                }
                checksum = (checksum * 31 + best_sad + best_dy * 17 + best_dx * 7) % 1000000007;
            }
        }
    }
    checksum
}

fn main() { println!("{}", motion_est_checksum(8)); }
