fn flocking_boids(boids: i64) -> i64 {
    let mut checksum: i64 = 0;
    let frames: i64 = 30;
    
    for f in 0..frames {
        let mut center_mass_sum: i64 = 0;
        for i in 0..boids {
            let px = (i * 17 + f * 7) % 1000;
            let py = (i * 19 + f * 11) % 1000;
            let vx = ((i * 23) % 20) - 10;
            let vy = ((i * 29) % 20) - 10;
            
            let mut sep_x: i64 = 0; let mut sep_y: i64 = 0;
            let mut align_x: i64 = 0; let mut align_y: i64 = 0;
            let mut coh_x: i64 = 0; let mut coh_y: i64 = 0;
            let mut neighbors: i64 = 0;
            
            for j in 0..boids {
                if i != j {
                    let ox = (j * 17 + f * 7) % 1000;
                    let oy = (j * 19 + f * 11) % 1000;
                    let ovx = ((j * 23) % 20) - 10;
                    let ovy = ((j * 29) % 20) - 10;
                    
                    let dx = px - ox;
                    let dy = py - oy;
                    let dist_sq = dx*dx + dy*dy;
                    if dist_sq < 2500 {
                        neighbors += 1;
                        if dist_sq < 400 { sep_x += dx; sep_y += dy; }
                        align_x += ovx; align_y += ovy;
                        coh_x += ox; coh_y += oy;
                    }
                }
            }
            
            let mut new_vx = vx; let mut new_vy = vy;
            if neighbors > 0 {
                align_x = (align_x / neighbors) - vx;
                align_y = (align_y / neighbors) - vy;
                coh_x = (coh_x / neighbors) - px;
                coh_y = (coh_y / neighbors) - py;
                
                new_vx = vx + (sep_x * 5 + align_x * 2 + coh_x * 1) / 100;
                new_vy = vy + (sep_y * 5 + align_y * 2 + coh_y * 1) / 100;
            }
            center_mass_sum = (center_mass_sum + new_vx + new_vy) % 1000000007;
        }
        checksum = (checksum * 31 + center_mass_sum) % 1000000007;
    }
    checksum
}

fn main() { println!("{}", flocking_boids(512)); }
