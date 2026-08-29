def branch_walk(depth: int, fanout: int) -> int:
    if depth == 0: return fanout
    return sum(branch_walk(depth - 1, fanout - 1) + c for c in range(fanout))
if __name__ == "__main__":
    print(branch_walk(5, 6))
