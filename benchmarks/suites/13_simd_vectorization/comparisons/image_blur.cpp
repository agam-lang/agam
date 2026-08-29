#include <cstdio>

static long long image_blur(long long size) {
    long long checksum = 0;
    for (long long y = 1; y < size - 1; ++y) {
        for (long long x = 1; x < size - 1; ++x) {
            long long p00 = (((y - 1) * size + (x - 1)) * 17 + 13) % 256;
            long long p01 = (((y - 1) * size + x) * 17 + 13) % 256;
            long long p02 = (((y - 1) * size + (x + 1)) * 17 + 13) % 256;
            
            long long p10 = ((y * size + (x - 1)) * 17 + 13) % 256;
            long long p11 = ((y * size + x) * 17 + 13) % 256;
            long long p12 = ((y * size + (x + 1)) * 17 + 13) % 256;
            
            long long p20 = (((y + 1) * size + (x - 1)) * 17 + 13) % 256;
            long long p21 = (((y + 1) * size + x) * 17 + 13) % 256;
            long long p22 = (((y + 1) * size + (x + 1)) * 17 + 13) % 256;
            
            long long sum = p00 + 2*p01 + p02 + 2*p10 + 4*p11 + 2*p12 + p20 + 2*p21 + p22;
            long long blur = sum / 16;
            checksum = (checksum * 31 + blur) % 1000000007LL;
        }
    }
    return checksum;
}

int main() { printf("%lld\n", image_blur(256)); return 0; }
