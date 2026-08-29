def matrix_checksum(size: int) -> int:
    s = 0
    for row in range(size):
        for col in range(size):
            cell = 0
            for inner in range(size):
                cell += ((row * inner) + 3) * ((inner * col) + 5)
            s += cell % 104729
    return s

if __name__ == "__main__":
    print(matrix_checksum(64))
