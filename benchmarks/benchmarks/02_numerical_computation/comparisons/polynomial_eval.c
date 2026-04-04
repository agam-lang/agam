#include <stdio.h>

static long long polynomial_cost(long long points, long long degree) {
    long long checksum = 0;
    for (long long point = 0; point < points; ++point) {
        long long x = (point % 97) + 3;
        long long value = 1;
        for (long long coeff = degree; coeff > 0; --coeff) {
            value = ((value * x) + ((coeff * 11) + (point % 29))) % 1000003;
        }
        checksum += value;
    }
    return checksum;
}

int main(void) {
    printf("%lld\n", polynomial_cost(800000, 16));
    return 0;
}
