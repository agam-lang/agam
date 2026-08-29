fn nbody_simulation(bodies: i64) -> i64 {
    let steps: i64 = 10;
    let mut checksum: i64 = 0;
    
    for step in 0..steps {
        for i in 0..bodies {
            let px = ((i * 17 + step) % 2000) - 1000;
            let py = ((i * 19 + step) % 2000) - 1000;
            let pz = ((i * 23 + step) % 2000) - 1000;
            
            let mut fx: i64 = 0; let mut fy: i64 = 0; let mut fz: i64 = 0;
            for j in 0..bodies {
                if i != j {
                    let ox = ((j * 17 + step) % 2000) - 1000;
                    let oy = ((j * 19 + step) % 2000) - 1000;
                    let oz = ((j * 23 + step) % 2000) - 1000;
                    
                    let dx = ox - px; let dy = oy - py; let dz = oz - pz;
                    let mut d_sq = dx*dx + dy*dy + dz*dz;
                    if d_sq == 0 { d_sq = 1; }
                    
                    let inv_dist_cubed = 1000000 / (d_sq * d_sq);
                    fx += dx * inv_dist_cubed;
                    fy += dy * inv_dist_cubed;
                    fz += dz * inv_dist_cubed;
                }
            }
            checksum = (checksum * 31 + fx + fy + fz) % 1000000007;
        }
    }
    checksum
}

fn main() { println!("{}", nbody_simulation(256)); }
