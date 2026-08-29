def chacha_quarter(a: int, b: int, c: int, d: int) -> int:
    mask = 4294967295
    av = (a + b) & mask
    dv = (((d ^ av) << 16) | (((d ^ av) & mask) >> 16)) & mask
    cv = (c + dv) & mask
    bv = (((b ^ cv) << 12) | (((b ^ cv) & mask) >> 20)) & mask
    av2 = (av + bv) & mask
    dv2 = (((dv ^ av2) << 8) | (((dv ^ av2) & mask) >> 24)) & mask
    cv2 = (cv + dv2) & mask
    bv2 = (((bv ^ cv2) << 7) | (((bv ^ cv2) & mask) >> 25)) & mask
    return (av2 + bv2 + cv2 + dv2) & mask

def chacha20_checksum(rounds: int) -> int:
    checksum = 0
    for i in range(rounds):
        s0, s1, s2, s3 = 1634760805, 857760878, 2036477234, 1797285236
        for r in range(10):
            q1 = chacha_quarter(s0, s1, s2, s3)
            q2 = chacha_quarter(s1 + i, s2 + r, s3, s0)
            q3 = chacha_quarter(s2, s3 + i, s0 + r, s1)
            q4 = chacha_quarter(s3, s0, s1 + i, s2 + r)
            s0, s1, s2, s3 = q1, q2, q3, q4
        checksum = (checksum * 37 + s0 + s1 + s2 + s3) % 1000000007
    return checksum

if __name__ == "__main__":
    print(chacha20_checksum(1024))
