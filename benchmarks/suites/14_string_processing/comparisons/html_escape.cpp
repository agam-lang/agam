#include <cstdio>

static long long html_escape(long long length) {
    long long checksum = 0;
    for (long long iter = 0; iter < 100; ++iter) {
        long long out_len = 0;
        for (long long i = 0; i < length; ++i) {
            long long c = ((i * 23 + iter * 11) % 10);
            if (c == 0) { out_len += 4; checksum = (checksum * 31 + 60) % 1000000007LL; }
            else if (c == 1) { out_len += 4; checksum = (checksum * 31 + 62) % 1000000007LL; }
            else if (c == 2) { out_len += 5; checksum = (checksum * 31 + 38) % 1000000007LL; }
            else if (c == 3) { out_len += 6; checksum = (checksum * 31 + 34) % 1000000007LL; }
            else if (c == 4) { out_len += 5; checksum = (checksum * 31 + 39) % 1000000007LL; }
            else { out_len += 1; checksum = (checksum * 31 + 97 + c) % 1000000007LL; }
        }
        checksum = (checksum * 37 + out_len) % 1000000007LL;
    }
    return checksum;
}

int main() { printf("%lld\n", html_escape(10000)); return 0; }
