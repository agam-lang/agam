#include <stdio.h>

static long long crc32_checksum(long long data_size) {
    long long crc = 4294967295LL;
    long long mask = 4294967295LL;
    long long poly = 3988292384LL;
    for (long long i = 0; i < data_size; ++i) {
        long long byte_val = (i * 1103515245LL + 12345LL) & 255;
        crc ^= byte_val;
        for (int bit = 0; bit < 8; ++bit) {
            if (crc & 1) crc = ((crc >> 1) & (mask >> 1)) ^ poly;
            else crc = (crc >> 1) & (mask >> 1);
        }
    }
    return (crc ^ mask) % 1000000007LL;
}

int main(void) { printf("%lld\n", crc32_checksum(16384)); return 0; }
