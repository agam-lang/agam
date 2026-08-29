#include <stdio.h>
static long long polynomial_cost(long long points, long long degree) {
    long long checksum = 0;
    for (long long p = 0; p < points; ++p) {
        long long x = (p % 97) + 3, value = 1;
        for (long long c = degree; c > 0; --c)
            value = ((value * x) + ((c * 11) + (p % 29))) % 1000003;
        checksum += value;
    }
    return checksum;
}
int main(void) { printf("%lld\n", polynomial_cost(800000, 16)); return 0; }
