fn clamp(val: i64) -> i64 {
    if val < 0 { 0 } else if val > 255 { 255 } else { val }
}

fn pixel_filter_checksum(width: i64, height: i64) -> i64 {
    let mut checksum: i64 = 0;
    for y in 1..(height - 1) {
        for x in 1..(width - 1) {
            let p00 = (((y - 1) * width + (x - 1)) * 17 + 13) % 256;
            let p01 = (((y - 1) * width + x) * 17 + 13) % 256;
            let p02 = (((y - 1) * width + (x + 1)) * 17 + 13) % 256;
            let p10 = ((y * width + (x - 1)) * 17 + 13) % 256;
            let p11 = ((y * width + x) * 17 + 13) % 256;
            let p12 = ((y * width + (x + 1)) * 17 + 13) % 256;
            let p20 = (((y + 1) * width + (x - 1)) * 17 + 13) % 256;
            let p21 = (((y + 1) * width + x) * 17 + 13) % 256;
            let p22 = (((y + 1) * width + (x + 1)) * 17 + 13) % 256;
            
            let blur = (p00 + 2*p01 + p02 + 2*p10 + 4*p11 + 2*p12 + p20 + 2*p21 + p22) / 16;
            let sobel_x = -p00 + p02 - 2*p10 + 2*p12 - p20 + p22;
            let sobel_y = -p00 - 2*p01 - p02 + p20 + 2*p21 + p22;
            let sobel = sobel_x * sobel_x + sobel_y * sobel_y;
            let edge = sobel / 100;
            
            let out = clamp(blur + edge);
            checksum = (checksum * 31 + out) % 1000000007;
        }
    }
    checksum
}

fn main() { println!("{}", pixel_filter_checksum(256, 256)); }
