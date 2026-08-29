#include <iostream>

int64_t render_scene_hits(int64_t rays_per_pixel, int64_t width, int64_t height) {
    int64_t hit_count = 0;
    for (int64_t y = 0; y < height; ++y) {
        for (int64_t x = 0; x < width; ++x) {
            for (int64_t r = 0; r < rays_per_pixel; ++r) {
                int64_t dx = x - (width / 2);
                int64_t dy = y - (height / 2);
                int64_t dz = 100;

                int64_t b = -5 * dz;
                int64_t c = 25 - 4;
                int64_t disc = (b * b) - ((dx * dx + dy * dy + dz * dz) * c);

                if (disc >= 0) {
                    hit_count += 1;
                }
            }
        }
    }
    return hit_count;
}

int main() {
    int64_t hits = render_scene_hits(16, 128, 128);
    std::cout << hits << std::endl;
    return 0;
}
