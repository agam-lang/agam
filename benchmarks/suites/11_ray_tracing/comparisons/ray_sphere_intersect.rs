fn ray_sphere_intersect(rays: i64) -> i64 {
    let mut checksum: i64 = 0;
    let sx: i64 = 500; let sy: i64 = 500; let sz: i64 = 1000; let radius: i64 = 300;
    let r2 = radius * radius;
    
    for r in 0..rays {
        let ox = ((r * 17) % 2000) - 1000;
        let oy = ((r * 31) % 2000) - 1000;
        let oz = -1000;
        
        let mut dx = ((r * 13) % 100) - 50;
        let mut dy = ((r * 19) % 100) - 50;
        let mut dz = 100;
        
        let inv_len = 10000 / (dx*dx + dy*dy + dz*dz + 1);
        dx = (dx * inv_len) / 100;
        dy = (dy * inv_len) / 100;
        dz = (dz * inv_len) / 100;
        
        let ocx = ox - sx;
        let ocy = oy - sy;
        let ocz = oz - sz;
        
        let b = 2 * (ocx * dx + ocy * dy + ocz * dz);
        let c = (ocx * ocx + ocy * ocy + ocz * ocz) - r2;
        let discriminant = (b * b) - (4 * c);
        
        let mut hit = false;
        if discriminant >= 0 {
            let t1 = (-b - discriminant/100) / 2;
            let t2 = (-b + discriminant/100) / 2;
            if t1 > 0 || t2 > 0 {
                hit = true;
                checksum = (checksum * 31 + t1 + t2) % 1000000007;
            }
        }
        if !hit {
            checksum = (checksum * 37 + 1) % 1000000007;
        }
    }
    checksum
}

fn main() { println!("{}", ray_sphere_intersect(8192)); }
