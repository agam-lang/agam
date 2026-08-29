def dot_product(size: int) -> int:
    sum_val = 0
    for i in range(size):
        a = ((i * 17 + 13) % 256) - 128
        b = ((i * 19 + 7) % 256) - 128
        sum_val += a * b
    return sum_val

if __name__ == "__main__":
    print(dot_product(65536))
