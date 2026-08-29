fn distance_sq(x1: i64, y1: i64, z1: i64, x2: i64, y2: i64, z2: i64) -> i64 {
    let dx = x2 - x1; let dy = y2 - y1; let dz = z2 - z1;
    dx*dx + dy*dy + dz*dz
}

fn photon_mapping(photons: i64) -> i64 {
    let queries: i64 = 1024;
    let mut checksum: i64 = 0;
    
    for q in 0..queries {
        let qx = (q * 17) % 1000;
        let qy = (q * 19) % 1000;
        let qz = (q * 23) % 1000;
        
        let mut gathered: i64 = 0;
        let radius_sq: i64 = 10000;
        
        for p in 0..photons {
            let px = (p * 31) % 1000;
            let py = (p * 37) % 1000;
            let pz = (p * 41) % 1000;
            
            let d2 = distance_sq(qx, qy, qz, px, py, pz);
            if d2 < radius_sq {
                let power = 1000 - (d2 * 1000) / radius_sq;
                gathered += power;
            }
        }
        checksum = (checksum * 31 + gathered) % 1000000007;
    }
    checksum
}

fn main() { println!("{}", photon_mapping(4096)); }
