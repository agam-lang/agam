def token_frequency_cost(tokens: int, buckets: int) -> int:
    total = 0
    for token in range(tokens):
        bucket = ((token * 19) + (token // 7)) % buckets
        if bucket < 8:
            total += (bucket * 7) + 3
        elif bucket < 24:
            total += (bucket * 3) + 1
        else:
            total += bucket + 11
    return total


if __name__ == "__main__":
    print(token_frequency_cost(7000000, 64))
