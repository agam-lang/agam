#include <stdio.h>

static long long token_frequency_cost(long long tokens, long long buckets) {
    long long total = 0;
    for (long long token = 0; token < tokens; ++token) {
        long long bucket = ((token * 19) + (token / 7)) % buckets;
        if (bucket < 8) {
            total += (bucket * 7) + 3;
        } else if (bucket < 24) {
            total += (bucket * 3) + 1;
        } else {
            total += bucket + 11;
        }
    }
    return total;
}

int main(void) {
    printf("%lld\n", token_frequency_cost(7000000, 64));
    return 0;
}
