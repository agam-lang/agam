def clamp(val: int) -> int:
    if val < 0: return 0
    if val > 255: return 255
    return val

def pixel_filter_checksum(width: int, height: int) -> int:
    checksum = 0
    for y in range(1, height - 1):
        for x in range(1, width - 1):
            p00 = (((y - 1) * width + (x - 1)) * 17 + 13) % 256
            p01 = (((y - 1) * width + x) * 17 + 13) % 256
            p02 = (((y - 1) * width + (x + 1)) * 17 + 13) % 256
            p10 = ((y * width + (x - 1)) * 17 + 13) % 256
            p11 = ((y * width + x) * 17 + 13) % 256
            p12 = ((y * width + (x + 1)) * 17 + 13) % 256
            p20 = (((y + 1) * width + (x - 1)) * 17 + 13) % 256
            p21 = (((y + 1) * width + x) * 17 + 13) % 256
            p22 = (((y + 1) * width + (x + 1)) * 17 + 13) % 256
            
            blur = (p00 + 2*p01 + p02 + 2*p10 + 4*p11 + 2*p12 + p20 + 2*p21 + p22) // 16
            sobel_x = -p00 + p02 - 2*p10 + 2*p12 - p20 + p22
            sobel_y = -p00 - 2*p01 - p02 + p20 + 2*p21 + p22
            sobel = sobel_x * sobel_x + sobel_y * sobel_y
            edge = sobel // 100
            
            out = clamp(blur + edge)
            checksum = (checksum * 31 + out) % 1000000007
    return checksum

if __name__ == "__main__":
    print(pixel_filter_checksum(256, 256))
