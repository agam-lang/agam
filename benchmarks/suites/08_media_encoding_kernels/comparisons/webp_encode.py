def paeth_predictor(left: int, top: int, top_left: int) -> int:
    p = left + top - top_left
    pa = abs(p - left)
    pb = abs(p - top)
    pc = abs(p - top_left)

    if pa <= pb and pa <= pc:
        return left
    if pb <= pc:
        return top
    return top_left

def webp_encode(width: int, height: int) -> int:
    mod_byte = 256
    mod_prime = 1000000007

    residual_sum = 0
    for y in range(1, height):
        for x in range(1, width):
            left = (((x - 1) * 17) + (y * 23)) % mod_byte
            top = ((x * 17) + ((y - 1) * 23)) % mod_byte
            top_left = (((x - 1) * 17) + ((y - 1) * 23)) % mod_byte
            current = ((x * 17) + (y * 23)) % mod_byte

            predicted = paeth_predictor(left, top, top_left)
            diff = abs(current - predicted)
            residual_sum = (residual_sum + diff) % mod_prime

    return residual_sum

if __name__ == "__main__":
    res = webp_encode(512, 512)
    print(res)
