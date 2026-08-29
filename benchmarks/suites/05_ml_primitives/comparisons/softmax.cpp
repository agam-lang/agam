#include <cstdio>
static long long softmax_like(long long width, long long rounds) {
    long long total = 0;
    for (long long r = 0; r < rounds; ++r) {
        long long partial = 0;
        for (long long l = 0; l < width; ++l)
            partial += ((l * 11) + r) % 251;
        total += partial;
    }
    return total;
}
int main() { printf("%lld\n", softmax_like(4096, 4000)); return 0; }
