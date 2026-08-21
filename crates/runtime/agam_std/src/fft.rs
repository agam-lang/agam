//! Fast Fourier Transform (FFT) and Signal Processing Routines.

use crate::complex::Complex;
use std::f64::consts::PI;

/// Compute 1D Fast Fourier Transform (Radix-2 Cooley-Tukey DIT).
///
/// If input length is not a power of two, it is zero-padded to the next power of two.
pub fn fft(input: &[Complex]) -> Vec<Complex> {
    let n = input.len();
    if n == 0 {
        return Vec::new();
    }

    let n_pow2 = n.next_power_of_two();
    let mut a: Vec<Complex> = input.to_vec();
    a.resize(n_pow2, Complex::ZERO);

    cooley_tukey_fft(&mut a, false);
    a
}

/// Compute 1D Inverse Fast Fourier Transform (IFFT) with $\frac{1}{N}$ scaling.
pub fn ifft(input: &[Complex]) -> Vec<Complex> {
    let n = input.len();
    if n == 0 {
        return Vec::new();
    }

    let n_pow2 = n.next_power_of_two();
    let mut a: Vec<Complex> = input.to_vec();
    a.resize(n_pow2, Complex::ZERO);

    cooley_tukey_fft(&mut a, true);

    let scale = 1.0 / (n_pow2 as f64);
    for val in &mut a {
        val.re *= scale;
        val.im *= scale;
    }

    a
}

fn cooley_tukey_fft(a: &mut [Complex], inverse: bool) {
    let n = a.len();
    if n <= 1 {
        return;
    }

    // Bit-reversal permutation
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            a.swap(i, j);
        }
    }

    // Butterfly computations
    let sign = if inverse { 1.0 } else { -1.0 };
    let mut len = 2;
    while len <= n {
        let angle = sign * 2.0 * PI / (len as f64);
        let wlen = Complex::new(angle.cos(), angle.sin());

        let mut i = 0;
        while i < n {
            let mut w = Complex::ONE;
            for k in 0..(len / 2) {
                let u = a[i + k];
                let v = a[i + k + len / 2].mul(w);

                a[i + k] = u.add(v);
                a[i + k + len / 2] = u.sub(v);

                w = w.mul(wlen);
            }
            i += len;
        }
        len <<= 1;
    }
}

/// Compute Hanning window of size $N$.
pub fn hanning_window(size: usize) -> Vec<f64> {
    if size == 0 {
        return Vec::new();
    }
    if size == 1 {
        return vec![1.0];
    }
    (0..size)
        .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f64 / (size - 1) as f64).cos()))
        .collect()
}

/// Compute Hamming window of size $N$.
pub fn hamming_window(size: usize) -> Vec<f64> {
    if size == 0 {
        return Vec::new();
    }
    if size == 1 {
        return vec![1.0];
    }
    (0..size)
        .map(|i| 0.54 - 0.46 * (2.0 * PI * i as f64 / (size - 1) as f64).cos())
        .collect()
}

/// Compute Blackman window of size $N$.
pub fn blackman_window(size: usize) -> Vec<f64> {
    if size == 0 {
        return Vec::new();
    }
    if size == 1 {
        return vec![1.0];
    }
    let a0 = 0.42;
    let a1 = 0.5;
    let a2 = 0.08;
    (0..size)
        .map(|i| {
            let theta = 2.0 * PI * i as f64 / (size - 1) as f64;
            a0 - a1 * theta.cos() + a2 * (2.0 * theta).cos()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fft_and_ifft_roundtrip() {
        let original = vec![
            Complex::new(1.0, 0.0),
            Complex::new(2.0, 0.0),
            Complex::new(3.0, 0.0),
            Complex::new(4.0, 0.0),
        ];

        let freq = fft(&original);
        assert_eq!(freq.len(), 4);
        // DC component (index 0) = sum of all values = 1+2+3+4 = 10.0
        assert!((freq[0].re - 10.0).abs() < 1e-6);

        let reconstructed = ifft(&freq);
        for (orig, rec) in original.iter().zip(reconstructed.iter()) {
            assert!((orig.re - rec.re).abs() < 1e-6);
            assert!((orig.im - rec.im).abs() < 1e-6);
        }
    }

    #[test]
    fn test_windowing_functions() {
        let hann = hanning_window(4);
        assert_eq!(hann.len(), 4);
        assert!((hann[0] - 0.0).abs() < 1e-6);
        assert!((hann[3] - 0.0).abs() < 1e-6);
    }
}
