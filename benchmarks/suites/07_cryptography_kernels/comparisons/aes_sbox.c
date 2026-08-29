#include <stdio.h>

static long long aes_sbox_val(long long input) {
    long long x = input & 255;
    long long inv = 1;
    for (int i = 0; i < 7; ++i) {
        inv = (inv * x) % 257;
        if (inv > 255) inv ^= 283;
    }
    inv &= 255;
    long long result = inv ^ ((inv << 1) & 255) ^ ((inv << 2) & 255) ^
                       ((inv << 3) & 255) ^ ((inv << 4) & 255) ^ 99;
    return result & 255;
}

static long long aes_checksum(long long blocks) {
    long long checksum = 0;
    for (long long b = 0; b < blocks; ++b) {
        long long state = 0;
        for (long long byte_idx = 0; byte_idx < 16; ++byte_idx) {
            long long input_byte = (b * 16 + byte_idx) * 1103515245LL + 12345LL;
            long long sub = aes_sbox_val(input_byte & 255);
            long long shifted = sub ^ ((byte_idx * 3) & 255);
            long long mixed = shifted * 2;
            if (mixed > 255) mixed ^= 283;
            state = (state * 31 + mixed) % 1000000007LL;
        }
        checksum = (checksum * 37 + state) % 1000000007LL;
    }
    return checksum;
}

int main(void) { printf("%lld\n", aes_checksum(512)); return 0; }
