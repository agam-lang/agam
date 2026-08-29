#include <cstdio>

static long long huffman_checksum(long long data_size) {
    long long checksum = 0, total_bits = 0;
    for (long long j = 0; j < data_size; ++j) {
        long long byte_val = (j * 7 + 13) % 256;
        long long depth = 1, val = byte_val;
        while (val > 1) { val /= 2; depth++; }
        long long code_len = depth;
        if (code_len < 2) code_len = 2;
        if (code_len > 12) code_len = 12;
        total_bits += code_len;
        long long code = byte_val % (1LL << code_len);
        checksum = (checksum * 37 + code * 19 + code_len * 7) % 1000000007;
    }
    checksum = (checksum * 53 + total_bits) % 1000000007;
    return checksum;
}

int main() { printf("%lld\n", huffman_checksum(4096)); return 0; }
