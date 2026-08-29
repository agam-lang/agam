#include <stdio.h>

static long long abs_val(long long v) { return v < 0 ? -v : v; }

static long long astar_pathfinding(long long grid_size) {
    long long max_nodes = 10000;
    long long checksum = 0;
    
    for (long long path_finds = 0; path_finds < 100; ++path_finds) {
        long long start_x = (path_finds * 17) % grid_size;
        long long start_y = (path_finds * 19) % grid_size;
        long long goal_x = (grid_size - 1) - (path_finds * 23) % grid_size;
        long long goal_y = (grid_size - 1) - (path_finds * 29) % grid_size;
        
        long long nodes_expanded = 0;
        long long curr_x = start_x, curr_y = start_y;
        long long path_len = 0;
        
        while ((curr_x != goal_x || curr_y != goal_y) && nodes_expanded < max_nodes) {
            nodes_expanded++;
            long long best_next_x = curr_x, best_next_y = curr_y, best_f = 999999999;
            
            for (long long dy = -1; dy <= 1; ++dy) {
                for (long long dx = -1; dx <= 1; ++dx) {
                    if (dx != 0 || dy != 0) {
                        long long nx = curr_x + dx, ny = curr_y + dy;
                        if (nx >= 0 && nx < grid_size && ny >= 0 && ny < grid_size) {
                            long long is_obstacle = ((nx * 31 + ny * 37) % 100 < 20);
                            if (!is_obstacle) {
                                long long g = path_len + 10 + (dx != 0 && dy != 0 ? 4 : 0);
                                long long h = (abs_val(nx - goal_x) + abs_val(ny - goal_y)) * 10;
                                long long f = g + h;
                                if (f < best_f) {
                                    best_f = f; best_next_x = nx; best_next_y = ny;
                                }
                            }
                        }
                    }
                }
            }
            if (best_next_x == curr_x && best_next_y == curr_y) {
                nodes_expanded = max_nodes; // stuck
            } else {
                curr_x = best_next_x; curr_y = best_next_y;
                path_len += 10;
            }
        }
        checksum = (checksum * 31 + path_len + nodes_expanded) % 1000000007LL;
    }
    return checksum;
}

int main(void) { printf("%lld\n", astar_pathfinding(100)); return 0; }
