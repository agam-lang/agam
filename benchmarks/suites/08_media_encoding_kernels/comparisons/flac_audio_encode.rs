fn flac_lpc_autocorrelation(samples: i64, _order: i64) -> i64 {
    let shift_scale: i64 = 4096;
    let mod_prime: i64 = 1000000007;

    let mut total_residual_energy: i64 = 0;
    let mut rice_bits_total: i64 = 0;

    let (qlp0, qlp1, qlp2, qlp3) = (1850i64, -1120i64, 680i64, -340i64);
    let (qlp4, qlp5, qlp6, qlp7) = (190i64, -95i64, 45i64, -18i64);

    let (mut h0, mut h1, mut h2, mut h3) = (0i64, 0i64, 0i64, 0i64);
    let (mut h4, mut h5, mut h6, mut h7) = (0i64, 0i64, 0i64, 0i64);

    for n in 0..samples {
        let phase1 = (n * 440) % 48000;
        let phase2 = (n * 880) % 48000;
        let sample = (((phase1 - 24000) * 32000) / 24000)
                   + (((phase2 - 24000) * 8000) / 24000);

        let predicted_acc = (qlp0 * h0) + (qlp1 * h1) + (qlp2 * h2) + (qlp3 * h3)
                          + (qlp4 * h4) + (qlp5 * h5) + (qlp6 * h6) + (qlp7 * h7);
        let predicted = predicted_acc / shift_scale;

        let mut residual = sample - predicted;
        if residual < 0 { residual = -residual; }

        total_residual_energy = (total_residual_energy + residual) % mod_prime;

        let rice_k = 4i64;
        let rice_bits = (residual / 16) + rice_k + 1;
        rice_bits_total = (rice_bits_total + rice_bits) % mod_prime;

        h7 = h6; h6 = h5; h5 = h4; h4 = h3; h3 = h2; h2 = h1; h1 = h0; h0 = sample;
    }

    (total_residual_energy * 31 + rice_bits_total) % mod_prime
}

fn main() {
    let res = flac_lpc_autocorrelation(100000, 8);
    println!("{}", res);
}
