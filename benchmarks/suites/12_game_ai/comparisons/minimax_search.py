def evaluate_board(state: int) -> int:
    score = 0
    for i in range(9):
        piece = (state >> (i * 2)) & 3
        if piece == 1: score += 10
        if piece == 2: score -= 10
    return score

def minimax(state: int, depth: int, is_max: bool) -> int:
    if depth == 0: return evaluate_board(state)
    
    if is_max:
        best_val = -999999
        for i in range(9):
            if ((state >> (i * 2)) & 3) == 0:
                next_state = state | (1 << (i * 2))
                val = minimax(next_state, depth - 1, False)
                if val > best_val: best_val = val
        return best_val if best_val != -999999 else evaluate_board(state)
    else:
        best_val = 999999
        for i in range(9):
            if ((state >> (i * 2)) & 3) == 0:
                next_state = state | (2 << (i * 2))
                val = minimax(next_state, depth - 1, True)
                if val < best_val: best_val = val
        return best_val if best_val != 999999 else evaluate_board(state)

def minimax_search(games: int) -> int:
    checksum = 0
    for g in range(games):
        initial_state = (g * 12345) % 262144
        result = minimax(initial_state, 6, True)
        checksum = (checksum * 31 + result) % 1000000007
    return checksum

if __name__ == "__main__":
    print(minimax_search(64))
