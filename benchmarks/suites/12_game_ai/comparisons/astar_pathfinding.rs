fn abs_val(v: i64) -> i64 { if v < 0 { -v } else { v } }

fn astar_pathfinding(grid_size: i64) -> i64 {
    let max_nodes: i64 = 10000;
    let mut checksum: i64 = 0;
    
    for path_finds in 0..100 {
        let start_x = (path_finds * 17) % grid_size;
        let start_y = (path_finds * 19) % grid_size;
        let goal_x = (grid_size - 1) - (path_finds * 23) % grid_size;
        let goal_y = (grid_size - 1) - (path_finds * 29) % grid_size;
        
        let mut nodes_expanded = 0;
        let mut curr_x = start_x;
        let mut curr_y = start_y;
        let mut path_len = 0;
        
        while (curr_x != goal_x || curr_y != goal_y) && nodes_expanded < max_nodes {
            nodes_expanded += 1;
            let mut best_next_x = curr_x;
            let mut best_next_y = curr_y;
            let mut best_f = 999999999;
            
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx != 0 || dy != 0 {
                        let nx = curr_x + dx;
                        let ny = curr_y + dy;
                        if nx >= 0 && nx < grid_size && ny >= 0 && ny < grid_size {
                            let is_obstacle = (nx * 31 + ny * 37) % 100 < 20;
                            if !is_obstacle {
                                let g = path_len + 10 + if dx != 0 && dy != 0 { 4 } else { 0 };
                                let h = (abs_val(nx - goal_x) + abs_val(ny - goal_y)) * 10;
                                let f = g + h;
                                if f < best_f {
                                    best_f = f; best_next_x = nx; best_next_y = ny;
                                }
                            }
                        }
                    }
                }
            }
            if best_next_x == curr_x && best_next_y == curr_y {
                nodes_expanded = max_nodes; // stuck
            } else {
                curr_x = best_next_x; curr_y = best_next_y;
                path_len += 10;
            }
        }
        checksum = (checksum * 31 + path_len + nodes_expanded) % 1000000007;
    }
    checksum
}

fn main() { println!("{}", astar_pathfinding(100)); }
