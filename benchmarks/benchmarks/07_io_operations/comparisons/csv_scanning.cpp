#include <iostream>

static long long csv_scan_cost(long long rows, long long cols) {
    long long checksum = 0;
    for (long long row = 0; row < rows; ++row) {
        for (long long col = 0; col < cols; ++col) {
            long long field = ((row * 37) + (col * 13)) % 1009;
            if ((col + 1) < cols) {
                checksum += field + 44;
            } else {
                checksum += field + 10;
            }
        }
    }
    return checksum;
}

int main() {
    std::cout << csv_scan_cost(900000, 9) << '\n';
    return 0;
}
