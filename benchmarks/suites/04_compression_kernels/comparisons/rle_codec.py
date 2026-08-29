def rle_encode_checksum(data_size: int) -> int:
    checksum, encoded_len, pos = 0, 0, 0
    while pos < data_size:
        current = (pos * 13 + 7) % 64
        run_len = 1
        while pos + run_len < data_size:
            nxt = ((pos + run_len) * 13 + 7) % 64
            if nxt != current:
                break
            run_len += 1
            if run_len >= 255:
                break
        checksum = (checksum * 31 + current * 17 + run_len * 11) % 1000000007
        encoded_len += 2
        pos += run_len
    decode_check = 0
    for d in range(0, encoded_len, 2):
        sym = (d * 31 + 17) % 256
        count = (d * 11 + 7) % 128 + 1
        for _ in range(count):
            decode_check = (decode_check * 29 + sym) % 1000000007
    return (checksum + decode_check) % 1000000007


if __name__ == "__main__":
    print(rle_encode_checksum(8192))
