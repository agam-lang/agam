fn zbuffer_rasterize(triangles: i64) -> i64 {
    let width: i64 = 256; let height: i64 = 256;
    let mut checksum: i64 = 0;
    
    for t in 0..triangles {
        let x0 = (t * 17) % width; let y0 = (t * 19) % height; let z0 = (t * 23) % 1000;
        let x1 = (x0 + 50) % width; let y1 = (y0 + 20) % height; let z1 = (t * 29) % 1000;
        let x2 = (x0 + 20) % width; let y2 = (y0 + 50) % height; let z2 = (t * 31) % 1000;
        
        let mut min_x = x0; if x1 < min_x { min_x = x1; } if x2 < min_x { min_x = x2; }
        let mut max_x = x0; if x1 > max_x { max_x = x1; } if x2 > max_x { max_x = x2; }
        let mut min_y = y0; if y1 < min_y { min_y = y1; } if y2 < min_y { min_y = y2; }
        let mut max_y = y0; if y1 > max_y { max_y = y1; } if y2 > max_y { max_y = y2; }
        
        let mut area = (x1 - x0) * (y2 - y0) - (x2 - x0) * (y1 - y0);
        if area == 0 { area = 1; }
        if area < 0 { area = -area; }
        
        let mut pixels_drawn: i64 = 0;
        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let w0 = (x1 - px) * (y2 - py) - (x2 - px) * (y1 - py);
                let w1 = (x2 - px) * (y0 - py) - (x0 - px) * (y2 - py);
                let w2 = (x0 - px) * (y1 - py) - (x1 - px) * (y0 - py);
                
                let mut is_inside = false;
                if w0 >= 0 && w1 >= 0 && w2 >= 0 { is_inside = true; }
                if w0 <= 0 && w1 <= 0 && w2 <= 0 { is_inside = true; }
                
                if is_inside {
                    let z = (w0 * z0 + w1 * z1 + w2 * z2) / area;
                    pixels_drawn += 1;
                    checksum = (checksum * 31 + z + px + py) % 1000000007;
                }
            }
        }
        checksum = (checksum * 37 + pixels_drawn) % 1000000007;
    }
    checksum
}

fn main() { println!("{}", zbuffer_rasterize(1024)); }
