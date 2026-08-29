def flac_lpc_autocorrelation(samples: int, order: int) -> int:
    shift_scale = 4096
    mod_prime = 1000000007

    total_residual_energy = 0
    rice_bits_total = 0

    qlp0, qlp1, qlp2, qlp3 = 1850, -1120, 680, -340
    qlp4, qlp5, qlp6, qlp7 = 190, -95, 45, -18

    h0 = h1 = h2 = h3 = h4 = h5 = h6 = h7 = 0

    for n in range(samples):
        phase1 = (n * 440) % 48000
        phase2 = (n * 880) % 48000
        t1 = int(((phase1 - 24000) * 32000) / 24000)
        t2 = int(((phase2 - 24000) * 8000) / 24000)
        sample = t1 + t2

        predicted_acc = (qlp0 * h0) + (qlp1 * h1) + (qlp2 * h2) + (qlp3 * h3) + (qlp4 * h4) + (qlp5 * h5) + (qlp6 * h6) + (qlp7 * h7)
        predicted = int(predicted_acc / shift_scale)

        residual = sample - predicted
        if residual < 0:
            residual = -residual

        total_residual_energy = (total_residual_energy + residual) % mod_prime

        rice_k = 4
        rice_bits = int(residual / 16) + rice_k + 1
        rice_bits_total = (rice_bits_total + rice_bits) % mod_prime

        h7, h6, h5, h4, h3, h2, h1, h0 = h6, h5, h4, h3, h2, h1, h0, sample

    return (total_residual_energy * 31 + rice_bits_total) % mod_prime

if __name__ == "__main__":
    res = flac_lpc_autocorrelation(100000, 8)
    print(res)
