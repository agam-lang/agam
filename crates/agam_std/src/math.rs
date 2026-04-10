//! High-performance mathematical functions.
//!
//! All algorithms use cache-friendly contiguous memory and minimal allocations.
//!
//! ## Numerical Integration
//! - Simpson's 1/3 rule (O(h⁴) error)
//! - Gaussian quadrature (5-point Legendre)
//!
//! ## Fast Fourier Transform
//! - Cooley-Tukey radix-2 FFT (O(n log n))
//!
//! ## Root Finding
//! - Bisection method (guaranteed convergence)
//! - Newton-Raphson (quadratic convergence)

/// Numerical integration via Simpson's 1/3 rule.
/// Integrates f(x) from a to b using n subdivisions (n must be even).
/// Error: O(h⁴) where h = (b-a)/n.
pub fn integrate_simpson<F: Fn(f64) -> f64>(f: &F, a: f64, b: f64, n: usize) -> f64 {
    let n = if n % 2 == 1 { n + 1 } else { n }; // ensure even
    let h = (b - a) / n as f64;
    let mut sum = f(a) + f(b);

    for i in 1..n {
        let x = a + i as f64 * h;
        sum += if i % 2 == 0 { 2.0 * f(x) } else { 4.0 * f(x) };
    }

    sum * h / 3.0
}

/// Gaussian quadrature (5-point Legendre).
/// Maps [a,b] → [-1,1] and applies 5-point Gauss-Legendre weights.
/// Exact for polynomials up to degree 9.
pub fn integrate_gauss5<F: Fn(f64) -> f64>(f: &F, a: f64, b: f64) -> f64 {
    // 5-point Gauss-Legendre nodes and weights on [-1, 1]
    const NODES: [f64; 5] = [
        -0.906179845938664,
        -0.538469310105683,
        0.0,
        0.538469310105683,
        0.906179845938664,
    ];
    const WEIGHTS: [f64; 5] = [
        0.236926885056189,
        0.478628670499366,
        0.568888888888889,
        0.478628670499366,
        0.236926885056189,
    ];

    let mid = 0.5 * (b + a);
    let half = 0.5 * (b - a);
    let mut sum = 0.0;
    for i in 0..5 {
        sum += WEIGHTS[i] * f(mid + half * NODES[i]);
    }
    sum * half
}

/// Cooley-Tukey radix-2 FFT (in-place).
/// Input: (real, imag) pairs. Length must be a power of 2.
/// Output: frequency-domain coefficients (in-place).
pub fn fft(real: &mut [f64], imag: &mut [f64]) {
    let n = real.len();
    assert_eq!(n, imag.len(), "real and imag must have same length");
    assert!(n.is_power_of_two(), "FFT length must be power of 2");

    // Bit-reversal permutation
    let mut j = 0usize;
    for i in 0..n {
        if i < j {
            real.swap(i, j);
            imag.swap(i, j);
        }
        let mut m = n >> 1;
        while m >= 1 && j >= m {
            j -= m;
            m >>= 1;
        }
        j += m;
    }

    // Butterfly passes
    let mut len = 2;
    while len <= n {
        let half = len / 2;
        let angle = -2.0 * std::f64::consts::PI / len as f64;
        let wn_r = angle.cos();
        let wn_i = angle.sin();

        let mut start = 0;
        while start < n {
            let mut w_r = 1.0;
            let mut w_i = 0.0;
            for k in 0..half {
                let u_r = real[start + k];
                let u_i = imag[start + k];
                let v_r = real[start + k + half] * w_r - imag[start + k + half] * w_i;
                let v_i = real[start + k + half] * w_i + imag[start + k + half] * w_r;
                real[start + k] = u_r + v_r;
                imag[start + k] = u_i + v_i;
                real[start + k + half] = u_r - v_r;
                imag[start + k + half] = u_i - v_i;
                let new_w_r = w_r * wn_r - w_i * wn_i;
                let new_w_i = w_r * wn_i + w_i * wn_r;
                w_r = new_w_r;
                w_i = new_w_i;
            }
            start += len;
        }
        len <<= 1;
    }
}

/// Inverse FFT — reconstructs signal from frequency domain.
pub fn ifft(real: &mut [f64], imag: &mut [f64]) {
    let n = real.len();
    // Conjugate
    for v in imag.iter_mut() {
        *v = -*v;
    }
    fft(real, imag);
    // Conjugate and scale
    let scale = 1.0 / n as f64;
    for v in real.iter_mut() {
        *v *= scale;
    }
    for v in imag.iter_mut() {
        *v = -*v * scale;
    }
}

/// Bisection root-finding: finds x where f(x) ≈ 0 in [a, b].
/// f(a) and f(b) must have opposite signs.
pub fn bisect<F: Fn(f64) -> f64>(f: &F, mut a: f64, mut b: f64, tol: f64, max_iter: usize) -> f64 {
    assert!(f(a) * f(b) < 0.0, "f(a) and f(b) must have opposite signs");
    for _ in 0..max_iter {
        let mid = 0.5 * (a + b);
        if (b - a) < tol {
            return mid;
        }
        if f(mid) * f(a) < 0.0 {
            b = mid;
        } else {
            a = mid;
        }
    }
    0.5 * (a + b)
}

/// Newton-Raphson root-finding: finds x where f(x) ≈ 0.
/// Requires f and f' (derivative). Quadratic convergence.
pub fn newton<F, FP>(f: &F, fp: &FP, mut x: f64, tol: f64, max_iter: usize) -> f64
where
    F: Fn(f64) -> f64,
    FP: Fn(f64) -> f64,
{
    for _ in 0..max_iter {
        let fx = f(x);
        if fx.abs() < tol {
            return x;
        }
        let fpx = fp(x);
        if fpx.abs() < 1e-15 {
            break;
        } // avoid division by zero
        x -= fx / fpx;
    }
    x
}

/// Gamma function approximation (Stirling's series for large x, Lanczos for small).
pub fn gamma(x: f64) -> f64 {
    if x < 0.5 {
        std::f64::consts::PI / ((std::f64::consts::PI * x).sin() * gamma(1.0 - x))
    } else {
        // Lanczos approximation (g=7)
        let coeffs = [
            0.99999999999980993,
            676.5203681218851,
            -1259.1392167224028,
            771.32342877765313,
            -176.61502916214059,
            12.507343278686905,
            -0.13857109526572012,
            9.9843695780195716e-6,
            1.5056327351493116e-7,
        ];
        let x = x - 1.0;
        let mut sum = coeffs[0];
        for i in 1..9 {
            sum += coeffs[i] / (x + i as f64);
        }
        let t = x + 7.5;
        (2.0 * std::f64::consts::PI).sqrt() * t.powf(x + 0.5) * (-t).exp() * sum
    }
}

/// Factorial (integer, exact for n ≤ 20).
pub fn factorial(n: u64) -> u64 {
    (1..=n).product()
}

/// Binomial coefficient C(n, k).
pub fn binomial(n: u64, k: u64) -> u64 {
    if k > n {
        return 0;
    }
    if k == 0 || k == n {
        return 1;
    }
    let k = k.min(n - k);
    let mut result = 1u64;
    for i in 0..k {
        result = result * (n - i) / (i + 1);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simpson_x_squared() {
        // ∫₀¹ x² dx = 1/3
        let result = integrate_simpson(&|x| x * x, 0.0, 1.0, 100);
        assert!((result - 1.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_simpson_sin() {
        // ∫₀^π sin(x) dx = 2
        let result = integrate_simpson(&|x| x.sin(), 0.0, std::f64::consts::PI, 100);
        assert!((result - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_gauss5_polynomial() {
        // ∫₀¹ x⁴ dx = 1/5 — Gauss-5 is exact for degree ≤ 9
        let result = integrate_gauss5(&|x| x.powi(4), 0.0, 1.0);
        assert!((result - 0.2).abs() < 1e-14);
    }

    #[test]
    fn test_fft_impulse() {
        // FFT of [1, 0, 0, 0] should give [1, 1, 1, 1]
        let mut real = vec![1.0, 0.0, 0.0, 0.0];
        let mut imag = vec![0.0, 0.0, 0.0, 0.0];
        fft(&mut real, &mut imag);
        for &v in &real {
            assert!((v - 1.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_fft_ifft_roundtrip() {
        let original = vec![1.0, 2.0, 3.0, 4.0];
        let mut real = original.clone();
        let mut imag = vec![0.0; 4];
        fft(&mut real, &mut imag);
        ifft(&mut real, &mut imag);
        for (a, b) in real.iter().zip(&original) {
            assert!((a - b).abs() < 1e-10);
        }
    }

    #[test]
    fn test_bisect_sqrt2() {
        // x² - 2 = 0 → x = √2
        let root = bisect(&|x| x * x - 2.0, 1.0, 2.0, 1e-12, 100);
        assert!((root - std::f64::consts::SQRT_2).abs() < 1e-10);
    }

    #[test]
    fn test_newton_cube_root() {
        // x³ - 27 = 0 → x = 3
        let root = newton(&|x| x * x * x - 27.0, &|x| 3.0 * x * x, 5.0, 1e-12, 50);
        assert!((root - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_gamma_factorial() {
        // Γ(n) = (n-1)! for integers
        assert!((gamma(5.0) - 24.0).abs() < 1e-8); // 4! = 24
        assert!((gamma(6.0) - 120.0).abs() < 1e-6); // 5! = 120
    }

    #[test]
    fn test_factorial() {
        assert_eq!(factorial(0), 1);
        assert_eq!(factorial(5), 120);
        assert_eq!(factorial(10), 3628800);
    }

    #[test]
    fn test_binomial() {
        assert_eq!(binomial(5, 2), 10);
        assert_eq!(binomial(10, 3), 120);
        assert_eq!(binomial(0, 0), 1);
    }
}
