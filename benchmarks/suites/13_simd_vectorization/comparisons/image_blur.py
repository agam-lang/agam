def image_blur(size: int) -> int:
    checksum = 0
    for y in range(1, size - 1):
        for x in range(1, size - 1):
            p00 = (((y - 1) * size + (x - 1)) * 17 + 13) % 256
            p01 = (((y - 1) * size + x) * 17 + 13) % 256
            p02 = (((y - 1) * size + (x + 1)) * 17 + 13) % 256
            
            p10 = ((y * size + (x - 1)) * 17 + 13) % 256
            p11 = ((y * size + x) * 17 + 13) % 256
            p12 = ((y * size + (x + 1)) * 17 + 13) % 256
            
            p20 = (((y + 1) * size + (x - 1)) * 17 + 13) % 256
            p21 = (((y + 1) * size + x) * 17 + 13) % 256
            p22 = (((y + 1) * size + (x + 1)) * 17 + 13) % 256
            
            sum_val = p00 + 2*p01 + p02 + 2*p10 + 4*p11 + 2*p12 + p20 + 2*p21 + p22
            blur = sum_val // 16
            checksum = (checksum * 31 + blur) % 1000000007
    return checksum

if __name__ == "__main__":
    print(image_blur(256))
