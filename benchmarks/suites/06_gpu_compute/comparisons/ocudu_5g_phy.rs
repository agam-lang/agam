fn ldpc_decode_layer(length: i64, iterations: i64) -> i64 {
    let lcg_m = 1664525i64;
    let lcg_c = 1013904223i64;
    let mod_byte = 256i64;
    let mid_val = 128i64;

    let mut syndrome_sum = 0i64;
    for _iter in 0..iterations {
        let mut i = 0i64;
        while i < length - 4 {
            let l0 = (((i * lcg_m) + lcg_c) % mod_byte) - mid_val;
            let l1 = ((((i + 1) * lcg_m) + lcg_c) % mod_byte) - mid_val;
            let l2 = ((((i + 2) * lcg_m) + lcg_c) % mod_byte) - mid_val;
            let l3 = ((((i + 3) * lcg_m) + lcg_c) % mod_byte) - mid_val;

            let a0 = l0.abs();
            let a1 = l1.abs();
            let a2 = l2.abs();
            let a3 = l3.abs();

            let check_val = a0.min(a1).min(a2).min(a3);
            let sign = if (l0 ^ l1 ^ l2 ^ l3) < 0 { -1 } else { 1 };

            syndrome_sum += check_val * sign;
            i += 4;
        }
    }
    syndrome_sum
}

fn main() {
    let pdsch_throughput = ldpc_decode_layer(16384, 8);
    println!("{}", pdsch_throughput);
}
