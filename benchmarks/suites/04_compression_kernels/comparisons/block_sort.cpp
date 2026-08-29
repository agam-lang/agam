#include <cstdio>

static long long block_sort_checksum(long long size) {
    long long checksum = 0;
    for (long long i = 0; i < size; ++i) {
        long long rot_checksum = 0;
        for (long long r = 0; r < size; ++r) {
            long long idx = (i + r) % size;
            long long val = (idx * 7 + 13) % 256;
            rot_checksum = (rot_checksum * 31 + val) % 1000000007;
        }
        checksum = (checksum + rot_checksum) % 1000000007;
    }
    long long sorted_check = 0;
    for (long long j = 0; j < size; ++j) {
        long long last_col = ((j + size - 1) % size * 7 + 13) % 256;
        sorted_check = (sorted_check * 37 + last_col * (j + 1)) % 1000000007;
    }
    return (checksum + sorted_check) % 1000000007;
}

int main() { printf("%lld\n", block_sort_checksum(256)); return 0; }
