#include <cstdio>
static long long pointer_chase(long long nodes, long long rounds) {
    long long cursor = 1, checksum = 0;
    for (long long s = 0; s < rounds; ++s) {
        cursor = ((cursor * 1103515245LL) + 12345) % nodes;
        checksum += cursor;
    }
    return checksum;
}
int main() { printf("%lld\n", pointer_chase(1000003, 8000000)); return 0; }
