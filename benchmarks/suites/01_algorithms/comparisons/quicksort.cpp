#include <cstdio>

static long long partition_cost(long long low, long long high, long long pivot) {
    long long score = 0;
    for (long long i = low; i < high; ++i) {
        long long probe = ((i * 17) + 13) % 997;
        if (probe < pivot) {
            score += probe;
        } else {
            score -= probe;
        }
    }
    return score;
}

static long long quicksort_cost(long long low, long long high) {
    if (high - low < 2) {
        return high - low;
    }
    long long pivot = (low + high) / 2;
    return partition_cost(low, high, pivot)
        + quicksort_cost(low, pivot)
        + quicksort_cost(pivot + 1, high);
}

int main() {
    printf("%lld\n", quicksort_cost(0, 5000));
    return 0;
}
