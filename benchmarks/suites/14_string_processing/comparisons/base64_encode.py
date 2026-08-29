def base64_encode(length: int) -> int:
    checksum = 0
    for iter_idx in range(100):
        for i in range(0, length, 3):
            b1 = ((i * 17 + iter_idx * 13) % 256)
            b2 = (((i + 1) * 19 + iter_idx * 7) % 256)
            b3 = (((i + 2) * 23 + iter_idx * 11) % 256)
            
            enc1 = (b1 >> 2) & 63
            enc2 = ((b1 & 3) << 4) | ((b2 >> 4) & 15)
            enc3 = ((b2 & 15) << 2) | ((b3 >> 6) & 3)
            enc4 = b3 & 63
            
            checksum = (checksum * 31 + enc1) % 1000000007
            checksum = (checksum * 31 + enc2) % 1000000007
            checksum = (checksum * 31 + enc3) % 1000000007
            checksum = (checksum * 31 + enc4) % 1000000007
    return checksum

if __name__ == "__main__":
    print(base64_encode(10000))
