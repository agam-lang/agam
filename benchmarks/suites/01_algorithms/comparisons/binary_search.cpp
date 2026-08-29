#include <cstdio>

static long long binary_search(long long limit, long long target) {
    long long low = 0;
    long long high = limit;
    while (low < high) {
        long long mid = low + ((high - low) / 2);
        long long value = (mid * 3) + 7;
        if (value == target) {
            return mid;
        }
        if (value < target) {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    return -1;
}

int main() {
    printf("%lld\n", binary_search(10000000, 2999998));
    return 0;
}
