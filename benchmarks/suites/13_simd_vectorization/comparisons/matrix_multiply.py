def matrix_multiply(size: int) -> int:
    checksum = 0
    s = size
    for i in range(s):
        for j in range(s):
            sum_val = 0
            for k in range(s):
                a = ((i * s + k) * 17 + 13) % 256
                b = ((k * s + j) * 19 + 7) % 256
                sum_val += a * b
            checksum = (checksum * 31 + sum_val) % 1000000007
    return checksum

if __name__ == "__main__":
    print(matrix_multiply(64))
