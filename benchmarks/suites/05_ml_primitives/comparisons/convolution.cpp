#include <cstdio>
static long long conv_score(long long w, long long h) {
    long long total = 0;
    for (long long y = 1; y < h - 1; ++y)
        for (long long x = 1; x < w - 1; ++x)
            total += ((x * 3) + (y * 5)) % 97;
    return total;
}
int main() { printf("%lld\n", conv_score(512, 512)); return 0; }
