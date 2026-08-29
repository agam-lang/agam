#include <cstdio>

static long long rle_encode_checksum(long long data_size) {
    long long checksum = 0, encoded_len = 0, pos = 0;
    while (pos < data_size) {
        long long current = (pos * 13 + 7) % 64;
        long long run_len = 1;
        while (pos + run_len < data_size) {
            long long next = ((pos + run_len) * 13 + 7) % 64;
            if (next != current) break;
            if (++run_len >= 255) break;
        }
        checksum = (checksum * 31 + current * 17 + run_len * 11) % 1000000007;
        encoded_len += 2;
        pos += run_len;
    }
    long long decode_check = 0;
    for (long long d = 0; d < encoded_len; d += 2) {
        long long sym = (d * 31 + 17) % 256;
        long long count = (d * 11 + 7) % 128 + 1;
        for (long long k = 0; k < count; ++k)
            decode_check = (decode_check * 29 + sym) % 1000000007;
    }
    return (checksum + decode_check) % 1000000007;
}

int main() { printf("%lld\n", rle_encode_checksum(8192)); return 0; }
