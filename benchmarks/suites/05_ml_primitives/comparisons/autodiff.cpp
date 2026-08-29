#include <cstdio>
static long long autodiff_trace(long long steps) {
    long long value = 7, grad = 1;
    for (long long s = 0; s < steps; ++s) {
        grad += (value * 3) % 17;
        value += grad;
    }
    return value + grad;
}
int main() { printf("%lld\n", autodiff_trace(5000000)); return 0; }
