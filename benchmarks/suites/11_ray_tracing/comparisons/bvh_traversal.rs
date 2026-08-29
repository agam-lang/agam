fn non_zero_dir(value: i64) -> i64 {
    if value >= 0 { value + 1 } else { value - 1 }
}

fn bvh_traversal(rays: i64) -> i64 {
    let nodes: i64 = 256;
    let mut checksum: i64 = 0;
    
    for r in 0..rays {
        let ox = (r * 17) % 1000;
        let oy = (r * 31) % 1000;
        let dx = ((r * 13) % 20) - 10;
        let dy = ((r * 19) % 20) - 10;
        let dx_div = non_zero_dir(dx);
        let dy_div = non_zero_dir(dy);
        
        let mut stack_size: i64 = 0;
        let mut current_node: i64 = 0;
        let mut hits: i64 = 0;
        
        let mut steps = 0;
        while steps < 100 {
            let min_x = (current_node * 7) % 1000;
            let max_x = min_x + 100;
            let min_y = (current_node * 11) % 1000;
            let max_y = min_y + 100;
            
            let mut tmin_x = (min_x - ox) * 100 / dx_div;
            let mut tmax_x = (max_x - ox) * 100 / dx_div;
            if tmin_x > tmax_x { let t = tmin_x; tmin_x = tmax_x; tmax_x = t; }
            
            let mut tmin_y = (min_y - oy) * 100 / dy_div;
            let mut tmax_y = (max_y - oy) * 100 / dy_div;
            if tmin_y > tmax_y { let t = tmin_y; tmin_y = tmax_y; tmax_y = t; }
            
            let tmin = if tmin_x > tmin_y { tmin_x } else { tmin_y };
            let tmax = if tmax_x < tmax_y { tmax_x } else { tmax_y };
            
            if tmax >= tmin && tmax > 0 {
                let is_leaf = current_node % 3 == 0;
                if is_leaf {
                    hits += 1;
                    stack_size -= 1;
                    if stack_size < 0 { steps = 999; }
                    current_node = (current_node * 2) % nodes;
                } else {
                    stack_size += 1;
                    current_node = (current_node * 2 + 1) % nodes;
                }
            } else {
                stack_size -= 1;
                if stack_size < 0 { steps = 999; }
                current_node = (current_node + 1) % nodes;
            }
            steps += 1;
        }
        checksum = (checksum * 31 + hits) % 1000000007;
    }
    checksum
}

fn main() { println!("{}", bvh_traversal(4096)); }
