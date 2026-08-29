def astar_pathfinding(grid_size: int) -> int:
    max_nodes = 10000
    checksum = 0
    
    for path_finds in range(100):
        start_x = (path_finds * 17) % grid_size
        start_y = (path_finds * 19) % grid_size
        goal_x = (grid_size - 1) - (path_finds * 23) % grid_size
        goal_y = (grid_size - 1) - (path_finds * 29) % grid_size
        
        nodes_expanded = 0
        curr_x, curr_y = start_x, start_y
        path_len = 0
        
        while (curr_x != goal_x or curr_y != goal_y) and nodes_expanded < max_nodes:
            nodes_expanded += 1
            best_next_x, best_next_y, best_f = curr_x, curr_y, 999999999
            
            for dy in range(-1, 2):
                for dx in range(-1, 2):
                    if dx != 0 or dy != 0:
                        nx, ny = curr_x + dx, curr_y + dy
                        if 0 <= nx < grid_size and 0 <= ny < grid_size:
                            is_obstacle = ((nx * 31 + ny * 37) % 100) < 20
                            if not is_obstacle:
                                g = path_len + 10 + (4 if dx != 0 and dy != 0 else 0)
                                h = (abs(nx - goal_x) + abs(ny - goal_y)) * 10
                                f = g + h
                                if f < best_f:
                                    best_f = f
                                    best_next_x, best_next_y = nx, ny
                                    
            if best_next_x == curr_x and best_next_y == curr_y:
                nodes_expanded = max_nodes # stuck
            else:
                curr_x, curr_y = best_next_x, best_next_y
                path_len += 10
                
        checksum = (checksum * 31 + path_len + nodes_expanded) % 1000000007
    return checksum

if __name__ == "__main__":
    print(astar_pathfinding(100))
