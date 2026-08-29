def abs_diff(a: int, b: int) -> int:
    return a - b if a > b else b - a

def c_mod(val: int, m: int) -> int:
    rem = abs(val) % m
    return -rem if val < 0 else rem

def motion_est_checksum(frames: int) -> int:
    width, height, block_size, search_range = 64, 64, 8, 4
    checksum = 0
    
    for f in range(frames):
        for by in range(height // block_size):
            for bx in range(width // block_size):
                best_sad = 999999999
                best_dy, best_dx = 0, 0
                for dy in range(-search_range, search_range + 1):
                    for dx in range(-search_range, search_range + 1):
                        sad = 0
                        for r in range(block_size):
                            for c in range(block_size):
                                cy = by * block_size + r
                                cx = bx * block_size + c
                                ry, rx = cy + dy, cx + dx
                                
                                cur_pixel = c_mod((f * width * height + cy * width + cx) * 17 + 13, 256)
                                ref_pixel = 0
                                if 0 <= ry < height and 0 <= rx < width:
                                    ref_pixel = c_mod(((f - 1) * width * height + ry * width + rx) * 17 + 13, 256)
                                sad += abs_diff(cur_pixel, ref_pixel)
                        if sad < best_sad:
                            best_sad = sad
                            best_dy = dy
                            best_dx = dx
                checksum = c_mod(checksum * 31 + best_sad + best_dy * 17 + best_dx * 7, 1000000007)
    return checksum

if __name__ == "__main__":
    print(motion_est_checksum(8))
