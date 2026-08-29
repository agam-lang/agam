#include <cstdio>

static long long chacha_quarter(long long a, long long b, long long c, long long d) {
    long long mask = 4294967295LL;
    long long av = (a + b) & mask;
    long long dv = ((d ^ av) << 16 | ((d ^ av) & mask) >> 16) & mask;
    long long cv = (c + dv) & mask;
    long long bv = ((b ^ cv) << 12 | ((b ^ cv) & mask) >> 20) & mask;
    long long av2 = (av + bv) & mask;
    long long dv2 = ((dv ^ av2) << 8 | ((dv ^ av2) & mask) >> 24) & mask;
    long long cv2 = (cv + dv2) & mask;
    long long bv2 = ((bv ^ cv2) << 7 | ((bv ^ cv2) & mask) >> 25) & mask;
    return (av2 + bv2 + cv2 + dv2) & mask;
}

static long long chacha20_checksum(long long rounds) {
    long long checksum = 0;
    for (long long i = 0; i < rounds; ++i) {
        long long s0 = 1634760805LL, s1 = 857760878LL, s2 = 2036477234LL, s3 = 1797285236LL;
        for (long long r = 0; r < 10; ++r) {
            long long q1 = chacha_quarter(s0, s1, s2, s3);
            long long q2 = chacha_quarter(s1 + i, s2 + r, s3, s0);
            long long q3 = chacha_quarter(s2, s3 + i, s0 + r, s1);
            long long q4 = chacha_quarter(s3, s0, s1 + i, s2 + r);
            s0 = q1; s1 = q2; s2 = q3; s3 = q4;
        }
        checksum = (checksum * 37 + s0 + s1 + s2 + s3) % 1000000007LL;
    }
    return checksum;
}

int main() { printf("%lld\n", chacha20_checksum(1024)); return 0; }
