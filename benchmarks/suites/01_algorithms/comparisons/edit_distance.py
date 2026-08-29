def edit_distance_cost(left: int, right: int) -> int:
    total = 0
    for row in range(left):
        for col in range(right):
            mismatch = ((row * 17) + (col * 13)) % 11
            if mismatch < 2:
                total += 1
            else:
                total += (row + col) % 3
    return total


if __name__ == "__main__":
    print(edit_distance_cost(2800, 2800))
