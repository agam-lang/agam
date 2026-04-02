#include <iostream>

static long long matrix_checksum(long long size) {
    long long total = 0;
    for (long long row = 0; row < size; ++row) {
        for (long long col = 0; col < size; ++col) {
            long long cell = 0;
            for (long long inner = 0; inner < size; ++inner) {
                cell += ((row * inner) + 3) * ((inner * col) + 5);
            }
            total += cell % 104729;
        }
    }
    return total;
}

int main() {
    std::cout << matrix_checksum(64) << '\n';
    return 0;
}

