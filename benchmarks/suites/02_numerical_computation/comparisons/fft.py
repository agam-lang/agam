def butterfly_cost(size: int) -> int:
    total, stage = 0, 1
    while stage < size:
        i = 0
        while i < size:
            left = ((i * 19) + stage) % 65521
            right = ((i * 23) + (stage * 3)) % 65521
            total += (left + right) - (left - right)
            i += stage * 2
        stage *= 2
    return total

if __name__ == "__main__":
    print(butterfly_cost(16384))
