def crc32_checksum(data_size: int) -> int:
    crc = 4294967295
    mask = 4294967295
    poly = 3988292384
    for i in range(data_size):
        byte_val = (i * 1103515245 + 12345) & 255
        crc ^= byte_val
        for _ in range(8):
            if crc & 1:
                crc = ((crc >> 1) & (mask >> 1)) ^ poly
            else:
                crc = (crc >> 1) & (mask >> 1)
    return (crc ^ mask) % 1000000007

if __name__ == "__main__":
    print(crc32_checksum(16384))
