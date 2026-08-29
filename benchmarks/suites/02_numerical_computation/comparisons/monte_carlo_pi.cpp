#include <cstdio>
static long long monte_carlo(long long samples) {
    long long inside = 0, seed = 123456789;
    for (long long i = 0; i < samples; ++i) {
        seed = ((seed * 1103515245LL) + 12345) % 2147483647;
        long long x = seed % 10000;
        seed = ((seed * 1103515245LL) + 12345) % 2147483647;
        long long y = seed % 10000;
        if ((x * x) + (y * y) <= 100000000) inside++;
    }
    return (inside * 4000000) / samples;
}
int main() { printf("%lld\n", monte_carlo(500000)); return 0; }
