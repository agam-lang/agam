#include <cstdio>

static long long non_zero_dir(long long value) {
    return value >= 0 ? value + 1 : value - 1;
}

static long long bvh_traversal(long long rays) {
    long long nodes = 256;
    long long checksum = 0;
    
    for (long long r = 0; r < rays; ++r) {
        long long ox = ((r * 17) % 1000);
        long long oy = ((r * 31) % 1000);
        long long dx = ((r * 13) % 20) - 10;
        long long dy = ((r * 19) % 20) - 10;
        long long dx_div = non_zero_dir(dx);
        long long dy_div = non_zero_dir(dy);
        
        long long stack_size = 0;
        long long current_node = 0;
        long long hits = 0;
        
        for (long long steps = 0; steps < 100; ++steps) {
            long long min_x = (current_node * 7) % 1000;
            long long max_x = min_x + 100;
            long long min_y = (current_node * 11) % 1000;
            long long max_y = min_y + 100;
            
            long long tmin_x = (min_x - ox) * 100 / dx_div;
            long long tmax_x = (max_x - ox) * 100 / dx_div;
            if (tmin_x > tmax_x) { long long t = tmin_x; tmin_x = tmax_x; tmax_x = t; }
            
            long long tmin_y = (min_y - oy) * 100 / dy_div;
            long long tmax_y = (max_y - oy) * 100 / dy_div;
            if (tmin_y > tmax_y) { long long t = tmin_y; tmin_y = tmax_y; tmax_y = t; }
            
            long long tmin = tmin_x > tmin_y ? tmin_x : tmin_y;
            long long tmax = tmax_x < tmax_y ? tmax_x : tmax_y;
            
            if (tmax >= tmin && tmax > 0) {
                long long is_leaf = (current_node % 3 == 0);
                if (is_leaf) {
                    hits++;
                    if (--stack_size < 0) steps = 999;
                    current_node = (current_node * 2) % nodes;
                } else {
                    stack_size++;
                    current_node = (current_node * 2 + 1) % nodes;
                }
            } else {
                if (--stack_size < 0) steps = 999;
                current_node = (current_node + 1) % nodes;
            }
        }
        checksum = (checksum * 31 + hits) % 1000000007LL;
    }
    return checksum;
}

int main() { printf("%lld\n", bvh_traversal(4096)); return 0; }
