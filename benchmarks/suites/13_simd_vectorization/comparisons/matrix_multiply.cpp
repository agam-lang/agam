#include <cstdio>

static long long matrix_multiply(long long size) {
    long long checksum = 0;
    long long s = size;
    for (long long i = 0; i < s; ++i) {
        for (long long j = 0; j < s; ++j) {
            long long sum = 0;
            for (long long k = 0; k < s; ++k) {
                long long a = ((i * s + k) * 17 + 13) % 256;
                long long b = ((k * s + j) * 19 + 7) % 256;
                sum += a * b;
            }
            checksum = (checksum * 31 + sum) % 1000000007LL;
        }
    }
    return checksum;
}

int main() { printf("%lld\n", matrix_multiply(64)); return 0; }
