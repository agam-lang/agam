#include <stdio.h>

static long long edit_distance_cost(long long left, long long right) {
    long long total = 0;
    for (long long row = 0; row < left; ++row) {
        for (long long col = 0; col < right; ++col) {
            long long mismatch = ((row * 17) + (col * 13)) % 11;
            if (mismatch < 2) {
                total += 1;
            } else {
                total += (row + col) % 3;
            }
        }
    }
    return total;
}

int main(void) {
    printf("%lld\n", edit_distance_cost(2800, 2800));
    return 0;
}
