fn image_blur(size: i64) -> i64 {
    let mut checksum: i64 = 0;
    for y in 1..(size - 1) {
        for x in 1..(size - 1) {
            let p00 = (((y - 1) * size + (x - 1)) * 17 + 13) % 256;
            let p01 = (((y - 1) * size + x) * 17 + 13) % 256;
            let p02 = (((y - 1) * size + (x + 1)) * 17 + 13) % 256;
            
            let p10 = ((y * size + (x - 1)) * 17 + 13) % 256;
            let p11 = ((y * size + x) * 17 + 13) % 256;
            let p12 = ((y * size + (x + 1)) * 17 + 13) % 256;
            
            let p20 = (((y + 1) * size + (x - 1)) * 17 + 13) % 256;
            let p21 = (((y + 1) * size + x) * 17 + 13) % 256;
            let p22 = (((y + 1) * size + (x + 1)) * 17 + 13) % 256;
            
            let sum = p00 + 2*p01 + p02 + 2*p10 + 4*p11 + 2*p12 + p20 + 2*p21 + p22;
            let blur = sum / 16;
            checksum = (checksum * 31 + blur) % 1000000007;
        }
    }
    checksum
}

fn main() { println!("{}", image_blur(256)); }
