fn liquid_dsp_fir(num_samples: i64, taps: i64) -> i64 {
    let mut accumulator: i64 = 0;
    for i in taps..num_samples {
        let mut sample_acc: i64 = 0;
        for k in 0..taps {
            let input_val = ((i - k) * 37) % 1000;
            let coeff = (k * 13) % 256;
            sample_acc += input_val * coeff;
        }
        accumulator = (accumulator + sample_acc) % 1000000007;
    }
    accumulator
}

fn main() {
    let res = liquid_dsp_fir(50000, 32);
    println!("{}", res);
}
