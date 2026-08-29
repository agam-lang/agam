#include <stdio.h>
#include <stdlib.h>
static const long long DEFAULT_SIZE = 96;
static long long matmul_score(long long size) {
    long long total = 0;
    for (long long row = 0; row < size; ++row)
        for (long long col = 0; col < size; ++col) {
            long long cell = 0;
            for (long long inner = 0; inner < size; ++inner)
                cell += ((row + inner) % 31) * ((inner + col) % 29);
            total += cell;
        }
    return total;
}
int main(int argc, char** argv) {
    const long long size = argc > 1 ? strtoll(argv[1], NULL, 10) : DEFAULT_SIZE;
    printf("%lld\n", matmul_score(size)); return 0;
}
