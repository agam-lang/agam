def liquid_dsp_fir(num_samples: int, taps: int) -> int:
    accumulator = 0
    for i in range(taps, num_samples):
        sample_acc = 0
        for k in range(taps):
            input_val = ((i - k) * 37) % 1000
            coeff = (k * 13) % 256
            sample_acc += input_val * coeff
        accumulator = (accumulator + sample_acc) % 1000000007
    return accumulator

if __name__ == "__main__":
    res = liquid_dsp_fir(50000, 32)
    print(res)
