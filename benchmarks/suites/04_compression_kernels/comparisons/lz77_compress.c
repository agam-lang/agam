#include <stdio.h>

static long long lz77_compress_checksum(long long data_size, long long window_size) {
    long long checksum = 0, pos = 0;
    while (pos < data_size) {
        long long best_len = 0, best_dist = 0;
        long long search_start = pos - window_size;
        if (search_start < 0) search_start = 0;
        for (long long s = search_start; s < pos; ++s) {
            long long match_len = 0;
            while (pos + match_len < data_size) {
                long long a = (s + match_len) * 7 + 13;
                long long b = (pos + match_len) * 7 + 13;
                if (a % 256 != b % 256) break;
                if (++match_len > 15) break;
            }
            if (match_len > best_len) { best_len = match_len; best_dist = pos - s; }
        }
        if (best_len >= 3) {
            checksum = (checksum * 31 + best_dist * 17 + best_len * 13) % 1000000007;
            pos += best_len;
        } else {
            long long literal = pos * 7 + 13;
            checksum = (checksum * 31 + literal % 256) % 1000000007;
            pos++;
        }
    }
    return checksum;
}

int main(void) { printf("%lld\n", lz77_compress_checksum(512, 32)); return 0; }
