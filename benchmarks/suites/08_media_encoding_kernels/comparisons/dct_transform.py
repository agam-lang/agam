def dct_checksum(blocks: int) -> int:
    checksum = 0
    for b in range(blocks):
        input_sum = 0
        output_sum = 0
        for u in range(8):
            for v in range(8):
                total_sum = 0
                for x in range(8):
                    for y in range(8):
                        pixel = ((b * 64 + x * 8 + y) * 17 + 13) % 256
                        if u == 0 and v == 0:
                            input_sum = (input_sum + pixel) % 1000000007
                        cos_x = ((2 * x + 1) * u * 314159) // 1600000
                        cos_y = ((2 * y + 1) * v * 314159) // 1600000
                        cx = (1000 * (100 - (cos_x * cos_x) // 20000)) // 100
                        cy = (1000 * (100 - (cos_y * cos_y) // 20000)) // 100
                        total_sum += (pixel * cx * cy) // 1000000
                cu = 707 if u == 0 else 1000
                cv = 707 if v == 0 else 1000
                dct_val = (total_sum * cu * cv) // 4000000
                output_sum = (output_sum * 31 + (dct_val + 10000)) % 1000000007
        checksum = (checksum * 37 + input_sum + output_sum) % 1000000007
    return checksum

if __name__ == "__main__":
    print(dct_checksum(64))
