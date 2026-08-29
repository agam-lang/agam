use std::cmp::{max, min};

fn evaluate_board(state: i64) -> i64 {
    let mut score: i64 = 0;
    for i in 0..9 {
        let piece = (state >> (i * 2)) & 3;
        if piece == 1 { score += 10; }
        if piece == 2 { score -= 10; }
    }
    score
}

fn minimax(state: i64, depth: i64, is_max: bool) -> i64 {
    if depth == 0 { return evaluate_board(state); }
    
    if is_max {
        let mut best_val = -999999;
        for i in 0..9 {
            if ((state >> (i * 2)) & 3) == 0 {
                let next_state = state | (1 << (i * 2));
                let val = minimax(next_state, depth - 1, false);
                best_val = max(best_val, val);
            }
        }
        if best_val == -999999 { evaluate_board(state) } else { best_val }
    } else {
        let mut best_val = 999999;
        for i in 0..9 {
            if ((state >> (i * 2)) & 3) == 0 {
                let next_state = state | (2 << (i * 2));
                let val = minimax(next_state, depth - 1, true);
                best_val = min(best_val, val);
            }
        }
        if best_val == 999999 { evaluate_board(state) } else { best_val }
    }
}

fn minimax_search(games: i64) -> i64 {
    let mut checksum: i64 = 0;
    for g in 0..games {
        let initial_state = (g * 12345) % 262144;
        let result = minimax(initial_state, 6, true);
        checksum = (checksum * 31 + result) % 1000000007;
    }
    checksum
}

fn main() { println!("{}", minimax_search(64)); }
