#include <stdio.h>

static long long regex_match(long long length) {
    long long checksum = 0;
    for (long long iter = 0; iter < 100; ++iter) {
        long long state = 0;
        long long matches = 0;
        for (long long i = 0; i < length; ++i) {
            long long c = ((i * 17 + iter * 13) % 5) + 97;
            if (state == 0) {
                if (c == 97) state = 0; else if (c == 98) state = 1; else state = 0;
            } else if (state == 1) {
                if (c == 98) state = 1; else if (c == 99) state = 2; else if (c == 100) state = 3; else if (c == 97) state = 0; else state = 0;
            } else if (state == 2) {
                if (c == 100) state = 3; else if (c == 97) state = 0; else state = 0;
            } else if (state == 3) {
                matches++;
                if (c == 97) state = 0; else if (c == 98) state = 1; else state = 0;
            }
        }
        checksum = (checksum * 31 + matches) % 1000000007LL;
    }
    return checksum;
}

int main(void) { printf("%lld\n", regex_match(10000)); return 0; }
