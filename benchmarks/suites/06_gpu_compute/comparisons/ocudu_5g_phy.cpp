#include <iostream>
#include <vector>
#include <cstdlib>
#include <algorithm>

int64_t ldpc_decode_layer(int64_t length, int64_t iterations) {
    const int64_t lcg_m = 1664525;
    const int64_t lcg_c = 1013904223;
    const int64_t mod_byte = 256;
    const int64_t mid_val = 128;

    int64_t syndrome_sum = 0;
    for (int64_t iter = 0; iter < iterations; ++iter) {
        for (int64_t i = 0; i < length - 4; i += 4) {
            int64_t l0 = (((i * lcg_m) + lcg_c) % mod_byte) - mid_val;
            int64_t l1 = ((((i + 1) * lcg_m) + lcg_c) % mod_byte) - mid_val;
            int64_t l2 = ((((i + 2) * lcg_m) + lcg_c) % mod_byte) - mid_val;
            int64_t l3 = ((((i + 3) * lcg_m) + lcg_c) % mod_byte) - mid_val;

            int64_t a0 = std::abs(l0);
            int64_t a1 = std::abs(l1);
            int64_t a2 = std::abs(l2);
            int64_t a3 = std::abs(l3);

            int64_t check_val = std::min({a0, a1, a2, a3});
            int64_t sign = ((l0 ^ l1 ^ l2 ^ l3) < 0) ? -1 : 1;

            syndrome_sum += check_val * sign;
        }
    }
    return syndrome_sum;
}

int main() {
    int64_t pdsch_throughput = ldpc_decode_layer(16384, 8);
    std::cout << pdsch_throughput << std::endl;
    return 0;
}
