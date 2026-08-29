#include <cstdio>
static long long branch_walk(long long depth, long long fanout) {
    if (depth == 0) return fanout;
    long long total = 0;
    for (long long c = 0; c < fanout; ++c)
        total += branch_walk(depth - 1, fanout - 1) + c;
    return total;
}
int main() { printf("%lld\n", branch_walk(5, 6)); return 0; }
