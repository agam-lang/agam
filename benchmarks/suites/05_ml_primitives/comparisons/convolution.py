def conv_score(w: int, h: int) -> int:
    total = 0
    for y in range(1, h - 1):
        for x in range(1, w - 1):
            total += ((x * 3) + (y * 5)) % 97
    return total
if __name__ == "__main__":
    print(conv_score(512, 512))
