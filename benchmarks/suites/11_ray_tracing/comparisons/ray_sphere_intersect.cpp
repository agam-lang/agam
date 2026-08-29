#include <cstdio>

static long long ray_sphere_intersect(long long rays) {
    long long checksum = 0;
    long long sx = 500, sy = 500, sz = 1000, radius = 300;
    long long r2 = radius * radius;
    
    for (long long r = 0; r < rays; ++r) {
        long long ox = ((r * 17) % 2000) - 1000;
        long long oy = ((r * 31) % 2000) - 1000;
        long long oz = -1000;
        
        long long dx = ((r * 13) % 100) - 50;
        long long dy = ((r * 19) % 100) - 50;
        long long dz = 100;
        
        long long inv_len = 10000 / (dx*dx + dy*dy + dz*dz + 1);
        dx = (dx * inv_len) / 100;
        dy = (dy * inv_len) / 100;
        dz = (dz * inv_len) / 100;
        
        long long ocx = ox - sx;
        long long ocy = oy - sy;
        long long ocz = oz - sz;
        
        long long b = 2 * (ocx * dx + ocy * dy + ocz * dz);
        long long c = (ocx * ocx + ocy * ocy + ocz * ocz) - r2;
        long long discriminant = (b * b) - (4 * c);
        
        long long hit = 0;
        if (discriminant >= 0) {
            long long t1 = (-b - discriminant/100) / 2;
            long long t2 = (-b + discriminant/100) / 2;
            if (t1 > 0 || t2 > 0) {
                hit = 1;
                checksum = (checksum * 31 + t1 + t2) % 1000000007LL;
            }
        }
        if (!hit) {
            checksum = (checksum * 37 + 1) % 1000000007LL;
        }
    }
    return checksum;
}

int main() { printf("%lld\n", ray_sphere_intersect(8192)); return 0; }
