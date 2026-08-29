#include <cstdio>

static long long zbuffer_rasterize(long long triangles) {
    long long width = 256, height = 256;
    long long checksum = 0;
    
    for (long long t = 0; t < triangles; ++t) {
        long long x0 = (t * 17) % width, y0 = (t * 19) % height, z0 = (t * 23) % 1000;
        long long x1 = (x0 + 50) % width, y1 = (y0 + 20) % height, z1 = (t * 29) % 1000;
        long long x2 = (x0 + 20) % width, y2 = (y0 + 50) % height, z2 = (t * 31) % 1000;
        
        long long min_x = x0; if (x1 < min_x) min_x = x1; if (x2 < min_x) min_x = x2;
        long long max_x = x0; if (x1 > max_x) max_x = x1; if (x2 > max_x) max_x = x2;
        long long min_y = y0; if (y1 < min_y) min_y = y1; if (y2 < min_y) min_y = y2;
        long long max_y = y0; if (y1 > max_y) max_y = y1; if (y2 > max_y) max_y = y2;
        
        long long area = (x1 - x0) * (y2 - y0) - (x2 - x0) * (y1 - y0);
        if (area == 0) area = 1;
        if (area < 0) area = -area;
        
        long long pixels_drawn = 0;
        for (long long py = min_y; py <= max_y; ++py) {
            for (long long px = min_x; px <= max_x; ++px) {
                long long w0 = (x1 - px) * (y2 - py) - (x2 - px) * (y1 - py);
                long long w1 = (x2 - px) * (y0 - py) - (x0 - px) * (y2 - py);
                long long w2 = (x0 - px) * (y1 - py) - (x1 - px) * (y0 - py);
                
                long long is_inside = 0;
                if (w0 >= 0 && w1 >= 0 && w2 >= 0) is_inside = 1;
                if (w0 <= 0 && w1 <= 0 && w2 <= 0) is_inside = 1;
                
                if (is_inside) {
                    long long z = (w0 * z0 + w1 * z1 + w2 * z2) / area;
                    pixels_drawn++;
                    checksum = (checksum * 31 + z + px + py) % 1000000007LL;
                }
            }
        }
        checksum = (checksum * 37 + pixels_drawn) % 1000000007LL;
    }
    return checksum;
}

int main() { printf("%lld\n", zbuffer_rasterize(1024)); return 0; }
