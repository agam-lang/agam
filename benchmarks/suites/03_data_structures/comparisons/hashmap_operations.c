#include <stdio.h>
static long long hashmap_probe_cost(long long slots, long long rounds) {
    long long total = 0;
    for (long long r = 0; r < rounds; ++r) {
        long long key = (r * 2654435761LL) % 2147483647;
        total += ((key % slots) * 37) % 4099;
    }
    return total;
}
int main(void) { printf("%lld\n", hashmap_probe_cost(65536, 5000000)); return 0; }
