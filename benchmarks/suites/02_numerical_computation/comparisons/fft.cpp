#include <cstdio>
static long long butterfly_cost(long long size) {
    long long total = 0, stage = 1;
    while (stage < size) {
        for (long long i = 0; i < size; i += stage * 2) {
            long long left = ((i * 19) + stage) % 65521;
            long long right = ((i * 23) + (stage * 3)) % 65521;
            total += (left + right) - (left - right);
        }
        stage *= 2;
    }
    return total;
}
int main() { printf("%lld\n", butterfly_cost(16384)); return 0; }
