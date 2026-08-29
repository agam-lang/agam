def ldpc_decode_layer(length: int, iterations: int) -> int:
    lcg_m = 1664525
    lcg_c = 1013904223
    mod_byte = 256
    mid_val = 128

    syndrome_sum = 0
    for _iter in range(iterations):
        for i in range(0, length - 4, 4):
            l0 = (((i * lcg_m) + lcg_c) % mod_byte) - mid_val
            l1 = ((((i + 1) * lcg_m) + lcg_c) % mod_byte) - mid_val
            l2 = ((((i + 2) * lcg_m) + lcg_c) % mod_byte) - mid_val
            l3 = ((((i + 3) * lcg_m) + lcg_c) % mod_byte) - mid_val

            a0 = abs(l0)
            a1 = abs(l1)
            a2 = abs(l2)
            a3 = abs(l3)

            check_val = min(a0, a1, a2, a3)
            sign = -1 if ((l0 ^ l1 ^ l2 ^ l3) < 0) else 1

            syndrome_sum += check_val * sign

    return syndrome_sum

if __name__ == "__main__":
    pdsch_throughput = ldpc_decode_layer(16384, 8)
    print(pdsch_throughput)
