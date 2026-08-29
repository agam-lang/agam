#include <stdio.h>

static long long abs_diff(long long a, long long b) { return a > b ? a - b : b - a; }

static long long motion_est_checksum(long long frames) {
    long long width = 64, height = 64, block_size = 8, search_range = 4;
    long long checksum = 0;
    
    for (long long f = 0; f < frames; ++f) {
        for (long long by = 0; by < height / block_size; ++by) {
            for (long long bx = 0; bx < width / block_size; ++bx) {
                long long best_sad = 999999999;
                long long best_dy = 0, best_dx = 0;
                
                for (long long dy = -search_range; dy <= search_range; ++dy) {
                    for (long long dx = -search_range; dx <= search_range; ++dx) {
                        long long sad = 0;
                        for (long long r = 0; r < block_size; ++r) {
                            for (long long c = 0; c < block_size; ++c) {
                                long long cy = by * block_size + r;
                                long long cx = bx * block_size + c;
                                long long ry = cy + dy, rx = cx + dx;
                                
                                long long cur_pixel = ((f * width * height + cy * width + cx) * 17 + 13) % 256;
                                long long ref_pixel = 0;
                                if (ry >= 0 && ry < height && rx >= 0 && rx < width) {
                                    ref_pixel = (((f - 1) * width * height + ry * width + rx) * 17 + 13) % 256;
                                }
                                sad += abs_diff(cur_pixel, ref_pixel);
                            }
                        }
                        if (sad < best_sad) { best_sad = sad; best_dy = dy; best_dx = dx; }
                    }
                }
                checksum = (checksum * 31 + best_sad + best_dy * 17 + best_dx * 7) % 1000000007;
            }
        }
    }
    return checksum;
}

int main(void) { printf("%lld\n", motion_est_checksum(8)); return 0; }
