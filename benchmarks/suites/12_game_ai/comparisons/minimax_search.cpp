#include <cstdio>

static long long max_val(long long a, long long b) { return a > b ? a : b; }
static long long min_val(long long a, long long b) { return a < b ? a : b; }

static long long evaluate_board(long long state) {
    long long score = 0;
    for (int i = 0; i < 9; ++i) {
        long long piece = (state >> (i * 2)) & 3;
        if (piece == 1) score += 10;
        if (piece == 2) score -= 10;
    }
    return score;
}

static long long minimax(long long state, long long depth, long long is_max) {
    if (depth == 0) return evaluate_board(state);
    
    if (is_max) {
        long long best_val = -999999;
        for (int i = 0; i < 9; ++i) {
            if (((state >> (i * 2)) & 3) == 0) {
                long long next_state = state | (1LL << (i * 2));
                long long val = minimax(next_state, depth - 1, 0);
                best_val = max_val(best_val, val);
            }
        }
        return best_val == -999999 ? evaluate_board(state) : best_val;
    } else {
        long long best_val = 999999;
        for (int i = 0; i < 9; ++i) {
            if (((state >> (i * 2)) & 3) == 0) {
                long long next_state = state | (2LL << (i * 2));
                long long val = minimax(next_state, depth - 1, 1);
                best_val = min_val(best_val, val);
            }
        }
        return best_val == 999999 ? evaluate_board(state) : best_val;
    }
}

static long long minimax_search(long long games) {
    long long checksum = 0;
    for (long long g = 0; g < games; ++g) {
        long long initial_state = (g * 12345) % 262144;
        long long result = minimax(initial_state, 6, 1);
        checksum = (checksum * 31 + result) % 1000000007LL;
    }
    return checksum;
}

int main() { printf("%lld\n", minimax_search(64)); return 0; }
