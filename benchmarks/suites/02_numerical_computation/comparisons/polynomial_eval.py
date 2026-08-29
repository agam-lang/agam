def polynomial_cost(points: int, degree: int) -> int:
    checksum = 0
    for p in range(points):
        x = (p % 97) + 3
        value = 1
        for c in range(degree, 0, -1):
            value = ((value * x) + ((c * 11) + (p % 29))) % 1000003
        checksum += value
    return checksum

if __name__ == "__main__":
    print(polynomial_cost(800000, 16))
