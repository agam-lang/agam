#include <cstdio>

static long long base64_encode(long long length) {
    long long checksum = 0;
    for (long long iter = 0; iter < 100; ++iter) {
        for (long long i = 0; i < length; i += 3) {
            long long b1 = ((i * 17 + iter * 13) % 256);
            long long b2 = (((i+1) * 19 + iter * 7) % 256);
            long long b3 = (((i+2) * 23 + iter * 11) % 256);
            
            long long enc1 = (b1 >> 2) & 63;
            long long enc2 = ((b1 & 3) << 4) | ((b2 >> 4) & 15);
            long long enc3 = ((b2 & 15) << 2) | ((b3 >> 6) & 3);
            long long enc4 = b3 & 63;
            
            checksum = (checksum * 31 + enc1) % 1000000007LL;
            checksum = (checksum * 31 + enc2) % 1000000007LL;
            checksum = (checksum * 31 + enc3) % 1000000007LL;
            checksum = (checksum * 31 + enc4) % 1000000007LL;
        }
    }
    return checksum;
}

int main() { printf("%lld\n", base64_encode(10000)); return 0; }
