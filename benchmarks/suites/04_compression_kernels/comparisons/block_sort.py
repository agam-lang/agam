def block_sort_checksum(size: int) -> int:
    checksum = 0
    for i in range(size):
        rot_checksum = 0
        for r in range(size):
            idx = (i + r) % size
            val = (idx * 7 + 13) % 256
            rot_checksum = (rot_checksum * 31 + val) % 1000000007
        checksum = (checksum + rot_checksum) % 1000000007
    sorted_check = 0
    for j in range(size):
        last_col = ((j + size - 1) % size * 7 + 13) % 256
        sorted_check = (sorted_check * 37 + last_col * (j + 1)) % 1000000007
    return (checksum + sorted_check) % 1000000007


if __name__ == "__main__":
    print(block_sort_checksum(256))
