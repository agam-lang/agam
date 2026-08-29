#include <cstdio>

static long long collision_detection(long long entities) {
    long long checksum = 0;
    long long frames = 60;
    
    for (long long f = 0; f < frames; ++f) {
        long long collisions = 0;
        for (long long i = 0; i < entities; ++i) {
            long long x1 = ((i * 17 + f * 13) % 1000) - 500;
            long long y1 = ((i * 19 + f * 11) % 1000) - 500;
            long long r1 = ((i * 23) % 40) + 10;
            
            for (long long j = i + 1; j < entities; ++j) {
                long long x2 = ((j * 17 + f * 13) % 1000) - 500;
                long long y2 = ((j * 19 + f * 11) % 1000) - 500;
                long long r2 = ((j * 23) % 40) + 10;
                
                long long dx = x2 - x1, dy = y2 - y1;
                long long dist_sq = dx * dx + dy * dy;
                long long rad_sum = r1 + r2;
                if (dist_sq <= rad_sum * rad_sum) {
                    collisions++;
                    checksum = (checksum * 31 + i + j) % 1000000007LL;
                }
            }
        }
        checksum = (checksum * 37 + collisions) % 1000000007LL;
    }
    return checksum;
}

int main() { printf("%lld\n", collision_detection(512)); return 0; }
