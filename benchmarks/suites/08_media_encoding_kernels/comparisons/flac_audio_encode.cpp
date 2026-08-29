#include <iostream>
#include <vector>

int64_t flac_lpc_autocorrelation(int64_t samples, int64_t order) {
    const int64_t shift_scale = 4096;
    const int64_t mod_prime = 1000000007;

    int64_t total_residual_energy = 0;
    int64_t rice_bits_total = 0;

    const int64_t qlp0 = 1850, qlp1 = -1120, qlp2 = 680, qlp3 = -340;
    const int64_t qlp4 = 190, qlp5 = -95, qlp6 = 45, qlp7 = -18;

    int64_t h0 = 0, h1 = 0, h2 = 0, h3 = 0, h4 = 0, h5 = 0, h6 = 0, h7 = 0;

    for (int64_t n = 0; n < samples; ++n) {
        int64_t phase1 = (n * 440) % 48000;
        int64_t phase2 = (n * 880) % 48000;
        int64_t sample = (((phase1 - 24000) * 32000) / 24000)
                       + (((phase2 - 24000) * 8000) / 24000);

        int64_t predicted_acc = (qlp0 * h0) + (qlp1 * h1) + (qlp2 * h2) + (qlp3 * h3)
                              + (qlp4 * h4) + (qlp5 * h5) + (qlp6 * h6) + (qlp7 * h7);
        int64_t predicted = predicted_acc / shift_scale;

        int64_t residual = sample - predicted;
        if (residual < 0) residual = -residual;

        total_residual_energy = (total_residual_energy + residual) % mod_prime;

        int64_t rice_k = 4;
        int64_t rice_bits = (residual / 16) + rice_k + 1;
        rice_bits_total = (rice_bits_total + rice_bits) % mod_prime;

        h7 = h6; h6 = h5; h5 = h4; h4 = h3; h3 = h2; h2 = h1; h1 = h0; h0 = sample;
    }

    return (total_residual_energy * 31 + rice_bits_total) % mod_prime;
}

int main() {
    int64_t res = flac_lpc_autocorrelation(100000, 8);
    std::cout << res << std::endl;
    return 0;
}
