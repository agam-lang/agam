#include <cstdio>

static long long distance_sq(long long x1, long long y1, long long z1, long long x2, long long y2, long long z2) {
    long long dx = x2 - x1, dy = y2 - y1, dz = z2 - z1;
    return dx*dx + dy*dy + dz*dz;
}

static long long photon_mapping(long long photons) {
    long long queries = 1024;
    long long checksum = 0;
    
    for (long long q = 0; q < queries; ++q) {
        long long qx = (q * 17) % 1000;
        long long qy = (q * 19) % 1000;
        long long qz = (q * 23) % 1000;
        
        long long gathered = 0;
        long long radius_sq = 10000;
        
        for (long long p = 0; p < photons; ++p) {
            long long px = (p * 31) % 1000;
            long long py = (p * 37) % 1000;
            long long pz = (p * 41) % 1000;
            
            long long d2 = distance_sq(qx, qy, qz, px, py, pz);
            if (d2 < radius_sq) {
                long long power = 1000 - (d2 * 1000) / radius_sq;
                gathered += power;
            }
        }
        checksum = (checksum * 31 + gathered) % 1000000007LL;
    }
    return checksum;
}

int main() { printf("%lld\n", photon_mapping(4096)); return 0; }
