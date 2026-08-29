def kvazaar_intra_predict(block_size: int) -> int:
    mod_byte = 256
    mod_prime = 1000000007

    checksum = 0
    for mode in range(35):
        pred_val = mode * 7
        for y in range(block_size):
            for x in range(block_size):
                pixel = ((x * 13) + (y * 17) + pred_val) % mod_byte
                checksum = ((checksum * 31) + pixel) % mod_prime

    return checksum

if __name__ == "__main__":
    res = kvazaar_intra_predict(32)
    print(res)
