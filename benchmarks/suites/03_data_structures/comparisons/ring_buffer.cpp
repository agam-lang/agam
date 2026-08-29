#include <cstdio>
static long long ring_buffer_cost(long long cap, long long rounds) {
    long long head = 0, tail = 0, acc = 0;
    for (long long item = 0; item < rounds; ++item) {
        long long slot = (head + item) % cap;
        acc += ((slot * 17) + item) % 257;
        if (item % 3 == 0) { tail = (tail + 1) % cap; acc += tail; }
        head = (head + 1) % cap;
    }
    return acc + head + tail;
}
int main() { printf("%lld\n", ring_buffer_cost(4096, 12000000)); return 0; }
