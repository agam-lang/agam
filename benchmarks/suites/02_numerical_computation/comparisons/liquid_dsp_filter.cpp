#include <iostream>
#include <vector>

int64_t liquid_dsp_fir(int64_t num_samples, int64_t taps) {
    int64_t accumulator = 0;
    for (int64_t i = taps; i < num_samples; ++i) {
        int64_t sample_acc = 0;
        for (int64_t k = 0; k < taps; ++k) {
            int64_t input_val = (i - k) * 37 % 1000;
            int64_t coeff = (k * 13) % 256;
            sample_acc += input_val * coeff;
        }
        accumulator = (accumulator + sample_acc) % 1000000007;
    }
    return accumulator;
}

int main() {
    int64_t res = liquid_dsp_fir(50000, 32);
    std::cout << res << std::endl;
    return 0;
}
