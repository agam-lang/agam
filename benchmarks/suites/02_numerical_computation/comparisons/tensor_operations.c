#include <stdio.h>
static long long tensor_checksum(long long w, long long h, long long d) {
    long long total = 0;
    for (long long z = 0; z < d; ++z)
        for (long long y = 0; y < h; ++y)
            for (long long x = 0; x < w; ++x)
                total += ((x * 31) + (y * 17) + (z * 13)) % 4093;
    return total;
}
int main(void) { printf("%lld\n", tensor_checksum(96, 96, 16)); return 0; }
