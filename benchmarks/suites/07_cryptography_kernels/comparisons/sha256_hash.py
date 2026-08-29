def rotr32(x: int, n: int) -> int:
    mask = 4294967295
    xm = x & mask
    return ((xm >> n) | (xm << (32 - n))) & mask

def sha256_checksum(blocks: int) -> int:
    h0, h1, h2, h3 = 1779033703, 3144134277, 1013904242, 2773480762
    h4, h5, h6, h7 = 1359893119, 2600822924, 528734635, 1541459225
    mask = 4294967295
    checksum = 0
    
    for b in range(blocks):
        a, bv, c, d = h0, h1, h2, h3
        e, f, g, hv = h4, h5, h6, h7
        for round in range(64):
            w = (b * 64 + round) * 1103515245 + 12345
            s1 = rotr32(e, 6) ^ rotr32(e, 11) ^ rotr32(e, 25)
            ch = (e & f) ^ ((~e) & g)
            temp1 = (hv + s1 + (ch & mask) + (w & mask) + round * 7) & mask
            s0 = rotr32(a, 2) ^ rotr32(a, 13) ^ rotr32(a, 22)
            maj = (a & bv) ^ (a & c) ^ (bv & c)
            temp2 = (s0 + (maj & mask)) & mask
            
            hv, g, f, e = g, f, e, (d + temp1) & mask
            d, c, bv, a = c, bv, a, (temp1 + temp2) & mask
        checksum = (checksum * 31 + a + bv + e + hv) % 1000000007
    return checksum

if __name__ == "__main__":
    print(sha256_checksum(256))
