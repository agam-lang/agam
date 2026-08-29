#include <cstdio>

static long long clamp(long long val) {
    if (val < 0) return 0;
    if (val > 255) return 255;
    return val;
}

static long long pixel_filter_checksum(long long width, long long height) {
    long long checksum = 0;
    for (long long y = 1; y < height - 1; ++y) {
        for (long long x = 1; x < width - 1; ++x) {
            long long p00 = (((y - 1) * width + (x - 1)) * 17 + 13) % 256;
            long long p01 = (((y - 1) * width + x) * 17 + 13) % 256;
            long long p02 = (((y - 1) * width + (x + 1)) * 17 + 13) % 256;
            long long p10 = ((y * width + (x - 1)) * 17 + 13) % 256;
            long long p11 = ((y * width + x) * 17 + 13) % 256;
            long long p12 = ((y * width + (x + 1)) * 17 + 13) % 256;
            long long p20 = (((y + 1) * width + (x - 1)) * 17 + 13) % 256;
            long long p21 = (((y + 1) * width + x) * 17 + 13) % 256;
            long long p22 = (((y + 1) * width + (x + 1)) * 17 + 13) % 256;
            
            long long blur = (p00 + 2*p01 + p02 + 2*p10 + 4*p11 + 2*p12 + p20 + 2*p21 + p22) / 16;
            long long sobel_x = -p00 + p02 - 2*p10 + 2*p12 - p20 + p22;
            long long sobel_y = -p00 - 2*p01 - p02 + p20 + 2*p21 + p22;
            long long sobel = sobel_x * sobel_x + sobel_y * sobel_y;
            long long edge = sobel / 100;
            
            long long out = clamp(blur + edge);
            checksum = (checksum * 31 + out) % 1000000007;
        }
    }
    return checksum;
}

int main() { printf("%lld\n", pixel_filter_checksum(256, 256)); return 0; }
