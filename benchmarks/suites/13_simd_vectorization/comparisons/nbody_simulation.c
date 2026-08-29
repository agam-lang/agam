#include <stdio.h>

static long long nbody_simulation(long long bodies) {
    long long steps = 10;
    long long checksum = 0;
    
    for (long long step = 0; step < steps; ++step) {
        for (long long i = 0; i < bodies; ++i) {
            long long px = ((i * 17 + step) % 2000) - 1000;
            long long py = ((i * 19 + step) % 2000) - 1000;
            long long pz = ((i * 23 + step) % 2000) - 1000;
            
            long long fx = 0, fy = 0, fz = 0;
            for (long long j = 0; j < bodies; ++j) {
                if (i != j) {
                    long long ox = ((j * 17 + step) % 2000) - 1000;
                    long long oy = ((j * 19 + step) % 2000) - 1000;
                    long long oz = ((j * 23 + step) % 2000) - 1000;
                    
                    long long dx = ox - px, dy = oy - py, dz = oz - pz;
                    long long d_sq = dx*dx + dy*dy + dz*dz;
                    if (d_sq == 0) d_sq = 1;
                    
                    long long inv_dist_cubed = 1000000 / (d_sq * d_sq);
                    fx += dx * inv_dist_cubed;
                    fy += dy * inv_dist_cubed;
                    fz += dz * inv_dist_cubed;
                }
            }
            checksum = (checksum * 31 + fx + fy + fz) % 1000000007LL;
        }
    }
    return checksum;
}

int main(void) { printf("%lld\n", nbody_simulation(256)); return 0; }
