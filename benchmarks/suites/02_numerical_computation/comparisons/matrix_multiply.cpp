#include <cstdio>
static long long matrix_checksum(long long size) {
    long long sum = 0;
    for (long long row = 0; row < size; ++row) {
        for (long long col = 0; col < size; ++col) {
            long long cell = 0;
            for (long long inner = 0; inner < size; ++inner) {
                cell += ((row * inner) + 3) * ((inner * col) + 5);
            }
            sum += cell % 104729;
        }
    }
    return sum;
}
int main() { printf("%lld\n", matrix_checksum(64)); return 0; }
