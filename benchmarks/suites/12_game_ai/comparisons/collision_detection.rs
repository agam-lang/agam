fn collision_detection(entities: i64) -> i64 {
    let mut checksum: i64 = 0;
    let frames: i64 = 60;
    
    for f in 0..frames {
        let mut collisions: i64 = 0;
        for i in 0..entities {
            let x1 = ((i * 17 + f * 13) % 1000) - 500;
            let y1 = ((i * 19 + f * 11) % 1000) - 500;
            let r1 = ((i * 23) % 40) + 10;
            
            for j in (i + 1)..entities {
                let x2 = ((j * 17 + f * 13) % 1000) - 500;
                let y2 = ((j * 19 + f * 11) % 1000) - 500;
                let r2 = ((j * 23) % 40) + 10;
                
                let dx = x2 - x1;
                let dy = y2 - y1;
                let dist_sq = dx * dx + dy * dy;
                let rad_sum = r1 + r2;
                
                if dist_sq <= rad_sum * rad_sum {
                    collisions += 1;
                    checksum = (checksum * 31 + i + j) % 1000000007;
                }
            }
        }
        checksum = (checksum * 37 + collisions) % 1000000007;
    }
    checksum
}

fn main() { println!("{}", collision_detection(512)); }
