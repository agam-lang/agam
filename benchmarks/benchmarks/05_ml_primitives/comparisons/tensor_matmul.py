def matmul_score(size: int) -> int:
    total = 0
    for row in range(size):
        for col in range(size):
            cell = 0
            for inner in range(size):
                cell += ((row + inner) % 31) * ((inner + col) % 29)
            total += cell
    return total


if __name__ == "__main__":
    print(matmul_score(48))

