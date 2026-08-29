#include <cstdio>

static long long dct_checksum(long long blocks) {
    long long checksum = 0;
    for (long long b = 0; b < blocks; ++b) {
        long long input_sum = 0;
        long long output_sum = 0;
        for (long long u = 0; u < 8; ++u) {
            for (long long v = 0; v < 8; ++v) {
                long long sum = 0;
                for (long long x = 0; x < 8; ++x) {
                    for (long long y = 0; y < 8; ++y) {
                        long long pixel = ((b * 64 + x * 8 + y) * 17 + 13) % 256;
                        if (u == 0 && v == 0) input_sum = (input_sum + pixel) % 1000000007;
                        long long cos_x = ((2 * x + 1) * u * 314159) / 1600000;
                        long long cos_y = ((2 * y + 1) * v * 314159) / 1600000;
                        long long cx = (1000 * (100 - (cos_x * cos_x) / 20000)) / 100;
                        long long cy = (1000 * (100 - (cos_y * cos_y) / 20000)) / 100;
                        sum += (pixel * cx * cy) / 1000000;
                    }
                }
                long long cu = (u == 0) ? 707 : 1000;
                long long cv = (v == 0) ? 707 : 1000;
                long long dct_val = (sum * cu * cv) / 4000000;
                output_sum = (output_sum * 31 + (dct_val + 10000)) % 1000000007;
            }
        }
        checksum = (checksum * 37 + input_sum + output_sum) % 1000000007;
    }
    return checksum;
}

int main() { printf("%lld\n", dct_checksum(64)); return 0; }
