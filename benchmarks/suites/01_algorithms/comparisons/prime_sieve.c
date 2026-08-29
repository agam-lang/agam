#include <stdio.h>

static int is_prime(long long n) {
    if (n < 2) return 0;
    for (long long d = 2; d * d <= n; ++d) {
        if (n % d == 0) return 0;
    }
    return 1;
}

static long long prime_count(long long limit) {
    long long total = 0;
    for (long long n = 2; n <= limit; ++n) {
        total += is_prime(n);
    }
    return total;
}

int main(void) {
    printf("%lld\n", prime_count(25000));
    return 0;
}
