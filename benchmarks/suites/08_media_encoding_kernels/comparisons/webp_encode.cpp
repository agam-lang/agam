#include <iostream>
#include <vector>
#include <cstdlib>

inline int64_t paeth_predictor(int64_t left, int64_t top, int64_t top_left) {
    int64_t p = left + top - top_left;
    int64_t pa = std::abs(p - left);
    int64_t pb = std::abs(p - top);
    int64_t pc = std::abs(p - top_left);

    if (pa <= pb && pa <= pc) return left;
    if (pb <= pc) return top;
    return top_left;
}

int64_t webp_encode(int64_t width, int64_t height) {
    const int64_t mod_byte = 256;
    const int64_t mod_prime = 1000000007;

    int64_t residual_sum = 0;
    for (int64_t y = 1; y < height; ++y) {
        for (int64_t x = 1; x < width; ++x) {
            int64_t left = (((x - 1) * 17) + (y * 23)) % mod_byte;
            int64_t top = ((x * 17) + ((y - 1) * 23)) % mod_byte;
            int64_t top_left = (((x - 1) * 17) + ((y - 1) * 23)) % mod_byte;
            int64_t current = ((x * 17) + (y * 23)) % mod_byte;

            int64_t predicted = paeth_predictor(left, top, top_left);
            int64_t diff = std::abs(current - predicted);
            residual_sum = (residual_sum + diff) % mod_prime;
        }
    }
    return residual_sum;
}

int main() {
    int64_t res = webp_encode(512, 512);
    std::cout << res << std::endl;
    return 0;
}
