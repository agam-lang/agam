#include <cstdio>

static long long json_parse(long long length) {
    long long checksum = 0;
    for (long long iter = 0; iter < 100; ++iter) {
        long long state = 0;
        long long elements = 0;
        long long depth = 0;
        for (long long i = 0; i < length; ++i) {
            long long c = ((i * 19 + iter * 7) % 6);
            if (c == 0) c = 123; else if (c == 1) c = 125; else if (c == 2) c = 34; else if (c == 3) c = 58; else if (c == 4) c = 44; else c = 97;
            
            if (state == 0) { if (c == 123) { state = 1; depth++; } }
            else if (state == 1) { if (c == 34) state = 2; else if (c == 125) { depth--; if (depth == 0) state = 0; else state = 6; } }
            else if (state == 2) { if (c == 34) state = 3; }
            else if (state == 3) { if (c == 58) state = 4; }
            else if (state == 4) { if (c == 34) state = 5; else if (c == 123) { state = 1; depth++; } }
            else if (state == 5) { if (c == 34) { state = 6; elements++; } }
            else if (state == 6) { if (c == 44) state = 1; else if (c == 125) { depth--; if (depth == 0) state = 0; } }
        }
        checksum = (checksum * 31 + elements + depth) % 1000000007LL;
    }
    return checksum;
}

int main() { printf("%lld\n", json_parse(10000)); return 0; }
