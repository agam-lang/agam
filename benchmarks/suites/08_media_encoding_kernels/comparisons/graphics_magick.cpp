#include <iostream>
#include <vector>
#include <algorithm>

inline int64_t min3(int64_t a, int64_t b, int64_t c) {
    return std::min({a, b, c});
}

inline int64_t max3(int64_t a, int64_t b, int64_t c) {
    return std::max({a, b, c});
}

int64_t graphics_magick_pipeline(int64_t dim) {
    const int64_t mod_byte = 256;
    const int64_t mod_prime = 1000000007;

    int64_t checksum = 0;

    for (int64_t y = 1; y < dim - 1; ++y) {
        for (int64_t x = 1; x < dim - 1; ++x) {
            int64_t r_c = (x * y * 17) % mod_byte;
            int64_t g_c = (x * y * 31) % mod_byte;
            int64_t b_c = (x * y * 47) % mod_byte;

            int64_t r_top = (x * (y - 1) * 17) % mod_byte;
            int64_t r_bot = (x * (y + 1) * 17) % mod_byte;
            int64_t r_left = ((x - 1) * y * 17) % mod_byte;
            int64_t r_right = ((x + 1) * y * 17) % mod_byte;

            int64_t r_tl = ((x - 1) * (y - 1) * 17) % mod_byte;
            int64_t r_tr = ((x + 1) * (y - 1) * 17) % mod_byte;
            int64_t r_bl = ((x - 1) * (y + 1) * 17) % mod_byte;
            int64_t r_br = ((x + 1) * (y + 1) * 17) % mod_byte;

            // Gaussian Sharpen Kernel: [-1 -2 -1; -2 13 -2; -1 -2 -1]
            int64_t conv_r = (r_c * 13)
                           - (r_top * 2) - (r_bot * 2) - (r_left * 2) - (r_right * 2)
                           - r_tl - r_tr - r_bl - r_br;

            int64_t sharp_r = std::clamp(conv_r, (int64_t)0, (int64_t)255);

            // RGB-to-HWB Color Space Transformation (magick/color.c)
            int64_t w_val = min3(sharp_r, g_c, b_c);
            int64_t v_val = max3(sharp_r, g_c, b_c);
            int64_t b_val = 255 - v_val;

            int64_t delta = v_val - w_val;
            int64_t hue = 0;
            if (delta > 0) {
                if (v_val == sharp_r) {
                    hue = ((g_c - b_c) * 60) / delta;
                } else if (v_val == g_c) {
                    hue = 120 + (((b_c - sharp_r) * 60) / delta);
                } else {
                    hue = 240 + (((sharp_r - g_c) * 60) / delta);
                }
                if (hue < 0) hue += 360;
            }

            // Swirl Coordinate Distortion Weighting
            int64_t dx = x - (dim / 2);
            int64_t dy = y - (dim / 2);
            int64_t r_sq = (dx * dx) + (dy * dy);
            int64_t swirl_factor = r_sq % 360;

            int64_t pixel_metric = sharp_r + (w_val * 2) + (b_val * 3) + hue + swirl_factor;
            checksum = (checksum + pixel_metric) % mod_prime;
        }
    }

    return checksum;
}

int main() {
    int64_t res = graphics_magick_pipeline(256);
    std::cout << res << std::endl;
    return 0;
}
