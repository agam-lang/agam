def aes_sbox_val(input: int) -> int:
    x = input & 255
    inv = 1
    for _ in range(7):
        inv = (inv * x) % 257
        if inv > 255:
            inv ^= 283
    inv &= 255
    result = inv ^ ((inv << 1) & 255) ^ ((inv << 2) & 255) ^ \
             ((inv << 3) & 255) ^ ((inv << 4) & 255) ^ 99
    return result & 255

def aes_checksum(blocks: int) -> int:
    checksum = 0
    for b in range(blocks):
        state = 0
        for byte_idx in range(16):
            input_byte = (b * 16 + byte_idx) * 1103515245 + 12345
            sub = aes_sbox_val(input_byte & 255)
            shifted = sub ^ ((byte_idx * 3) & 255)
            mixed = shifted * 2
            if mixed > 255:
                mixed ^= 283
            state = (state * 31 + mixed) % 1000000007
        checksum = (checksum * 37 + state) % 1000000007
    return checksum

if __name__ == "__main__":
    print(aes_checksum(512))
