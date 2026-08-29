def tensor_checksum(w: int, h: int, d: int) -> int:
    total = 0
    for z in range(d):
        for y in range(h):
            for x in range(w):
                total += ((x * 31) + (y * 17) + (z * 13)) % 4093
    return total

if __name__ == "__main__":
    print(tensor_checksum(96, 96, 16))
