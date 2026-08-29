def huffman_checksum(data_size: int) -> int:
    checksum, total_bits = 0, 0
    for j in range(data_size):
        byte_val = (j * 7 + 13) % 256
        depth, val = 1, byte_val
        while val > 1:
            val //= 2
            depth += 1
        code_len = depth
        if code_len < 2: code_len = 2
        if code_len > 12: code_len = 12
        total_bits += code_len
        code = byte_val % (1 << code_len)
        checksum = (checksum * 37 + code * 19 + code_len * 7) % 1000000007
    return (checksum * 53 + total_bits) % 1000000007


if __name__ == "__main__":
    print(huffman_checksum(4096))
