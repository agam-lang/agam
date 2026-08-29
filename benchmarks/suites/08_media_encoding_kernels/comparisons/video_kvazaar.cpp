#include <iostream>
#include <vector>
#include <chrono>

int64_t kvazaar_intra_predict(int64_t block_size) {
    int64_t checksum = 0;
    for (int64_t mode = 0; mode < 35; ++mode) {
        int64_t pred_val = mode * 7;
        for (int64_t y = 0; y < block_size; ++y) {
            for (int64_t x = 0; x < block_size; ++x) {
                int64_t pixel = (x * 13 + y * 17 + pred_val) % 256;
                checksum = (checksum * 31 + pixel) % 1000000007;
            }
        }
    }
    return checksum;
}

int main() {
    int64_t res = kvazaar_intra_predict(32);
    std::cout << res << std::endl;
    return 0;
}
