def matrix_checksum(size: int) -> int:
    total = 0
    for row in range(size):
        for col in range(size):
            cell = 0
            for inner in range(size):
                cell += ((row * inner) + 3) * ((inner * col) + 5)
            total += cell % 104729
    return total


if __name__ == "__main__":
    print(matrix_checksum(64))

