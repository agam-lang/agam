def polynomial_cost(points: int, degree: int) -> int:
    checksum = 0
    for point in range(points):
        x = (point % 97) + 3
        value = 1
        for coeff in range(degree, 0, -1):
            value = ((value * x) + ((coeff * 11) + (point % 29))) % 1_000_003
        checksum += value
    return checksum


if __name__ == "__main__":
    print(polynomial_cost(800000, 16))
