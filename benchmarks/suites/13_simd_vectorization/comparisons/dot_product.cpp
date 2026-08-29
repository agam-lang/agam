#include <cstdio>

static long long dot_product(long long size) {
    long long sum = 0;
    for (long long i = 0; i < size; ++i) {
        long long a = ((i * 17 + 13) % 256) - 128;
        long long b = ((i * 19 + 7) % 256) - 128;
        sum += a * b;
    }
    return sum;
}

int main() { printf("%lld\n", dot_product(65536)); return 0; }
