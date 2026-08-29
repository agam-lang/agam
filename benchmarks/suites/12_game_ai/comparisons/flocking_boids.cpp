#include <cstdio>

static long long flocking_boids(long long boids) {
    long long checksum = 0;
    long long frames = 30;
    
    for (long long f = 0; f < frames; ++f) {
        long long center_mass_sum = 0;
        for (long long i = 0; i < boids; ++i) {
            long long px = ((i * 17 + f * 7) % 1000);
            long long py = ((i * 19 + f * 11) % 1000);
            long long vx = ((i * 23) % 20) - 10;
            long long vy = ((i * 29) % 20) - 10;
            
            long long sep_x = 0, sep_y = 0;
            long long align_x = 0, align_y = 0;
            long long coh_x = 0, coh_y = 0;
            long long neighbors = 0;
            
            for (long long j = 0; j < boids; ++j) {
                if (i != j) {
                    long long ox = ((j * 17 + f * 7) % 1000);
                    long long oy = ((j * 19 + f * 11) % 1000);
                    long long ovx = ((j * 23) % 20) - 10;
                    long long ovy = ((j * 29) % 20) - 10;
                    
                    long long dx = px - ox, dy = py - oy;
                    long long dist_sq = dx*dx + dy*dy;
                    if (dist_sq < 2500) {
                        neighbors++;
                        if (dist_sq < 400) { sep_x += dx; sep_y += dy; }
                        align_x += ovx; align_y += ovy;
                        coh_x += ox; coh_y += oy;
                    }
                }
            }
            
            long long new_vx = vx, new_vy = vy;
            if (neighbors > 0) {
                align_x = (align_x / neighbors) - vx;
                align_y = (align_y / neighbors) - vy;
                coh_x = (coh_x / neighbors) - px;
                coh_y = (coh_y / neighbors) - py;
                
                new_vx = vx + (sep_x * 5 + align_x * 2 + coh_x * 1) / 100;
                new_vy = vy + (sep_y * 5 + align_y * 2 + coh_y * 1) / 100;
            }
            center_mass_sum = (center_mass_sum + new_vx + new_vy) % 1000000007LL;
        }
        checksum = (checksum * 31 + center_mass_sum) % 1000000007LL;
    }
    return checksum;
}

int main() { printf("%lld\n", flocking_boids(512)); return 0; }
