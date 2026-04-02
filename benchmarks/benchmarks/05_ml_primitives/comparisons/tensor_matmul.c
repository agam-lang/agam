#include <stdio.h>

static long long matmul_score(long long size) {
    long long total = 0;
    for (long long row = 0; row < size; ++row) {
        for (long long col = 0; col < size; ++col) {
            long long cell = 0;
            for (long long inner = 0; inner < size; ++inner) {
                cell += ((row + inner) % 31) * ((inner + col) % 29);
            }
            total += cell;
        }
    }
    return total;
}

int main(void) {
    printf("%lld\n", matmul_score(48));
    return 0;
}
