fn mandelbrot_set(size: i64) -> i64 {
    let max_iter: i64 = 100;
    let mut checksum: i64 = 0;
    
    for py in 0..size {
        for px in 0..size {
            let x0 = (px * 3500) / size - 2500;
            let y0 = (py * 2000) / size - 1000;
            
            let mut x: i64 = 0; let mut y: i64 = 0; let mut iter: i64 = 0;
            while x*x + y*y <= 4000000 && iter < max_iter {
                let xtemp = (x*x - y*y) / 1000 + x0;
                y = (2*x*y) / 1000 + y0;
                x = xtemp;
                iter += 1;
            }
            checksum = (checksum * 31 + iter) % 1000000007;
        }
    }
    checksum
}

fn main() { println!("{}", mandelbrot_set(256)); }
