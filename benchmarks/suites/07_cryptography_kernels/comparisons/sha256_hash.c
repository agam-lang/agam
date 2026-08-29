#include <stdio.h>

static long long rotr32(long long x, long long n) {
    long long mask = 4294967295LL;
    long long xm = x & mask;
    return ((xm >> n) | (xm << (32 - n))) & mask;
}

static long long sha256_checksum(long long blocks) {
    long long h0 = 1779033703LL, h1 = 3144134277LL, h2 = 1013904242LL, h3 = 2773480762LL;
    long long h4 = 1359893119LL, h5 = 2600822924LL, h6 = 528734635LL, h7 = 1541459225LL;
    long long mask = 4294967295LL;
    long long checksum = 0;
    
    for (long long b = 0; b < blocks; ++b) {
        long long a = h0, bv = h1, c = h2, d = h3, e = h4, f = h5, g = h6, hv = h7;
        for (long long round = 0; round < 64; ++round) {
            long long w = (b * 64 + round) * 1103515245LL + 12345LL;
            long long s1 = rotr32(e, 6) ^ rotr32(e, 11) ^ rotr32(e, 25);
            long long ch = (e & f) ^ ((~e) & g);
            long long temp1 = (hv + s1 + (ch & mask) + (w & mask) + round * 7) & mask;
            long long s0 = rotr32(a, 2) ^ rotr32(a, 13) ^ rotr32(a, 22);
            long long maj = (a & bv) ^ (a & c) ^ (bv & c);
            long long temp2 = (s0 + (maj & mask)) & mask;
            
            hv = g; g = f; f = e; e = (d + temp1) & mask;
            d = c; c = bv; bv = a; a = (temp1 + temp2) & mask;
        }
        checksum = (checksum * 31 + a + bv + e + hv) % 1000000007LL;
    }
    return checksum;
}

int main(void) { printf("%lld\n", sha256_checksum(256)); return 0; }
