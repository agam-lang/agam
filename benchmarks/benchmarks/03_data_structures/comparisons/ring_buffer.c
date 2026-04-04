#include <stdio.h>

static long long ring_buffer_cost(long long capacity, long long rounds) {
    long long head = 0;
    long long tail = 0;
    long long acc = 0;
    for (long long item = 0; item < rounds; ++item) {
        long long slot = (head + item) % capacity;
        acc += ((slot * 17) + item) % 257;
        if ((item % 3) == 0) {
            tail = (tail + 1) % capacity;
            acc += tail;
        }
        head = (head + 1) % capacity;
    }
    return acc + head + tail;
}

int main(void) {
    printf("%lld\n", ring_buffer_cost(4096, 12000000));
    return 0;
}
