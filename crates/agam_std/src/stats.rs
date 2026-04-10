//! Statistics and probability — hardware-optimized.
//!
//! Provides:
//! - Descriptive statistics (mean, variance, median, percentiles)
//! - PRNG (xoshiro256** — fast, high-quality)
//! - Probability distributions (Normal, Uniform)
//! - Hypothesis testing (t-test)

/// Descriptive statistics on a contiguous f64 slice.
pub struct Stats;

impl Stats {
    pub fn mean(data: &[f64]) -> f64 {
        data.iter().sum::<f64>() / data.len() as f64
    }

    pub fn variance(data: &[f64]) -> f64 {
        let mu = Self::mean(data);
        data.iter().map(|x| (x - mu) * (x - mu)).sum::<f64>() / data.len() as f64
    }

    pub fn std_dev(data: &[f64]) -> f64 {
        Self::variance(data).sqrt()
    }

    /// Sample variance (Bessel's correction: n-1 denominator).
    pub fn sample_variance(data: &[f64]) -> f64 {
        let mu = Self::mean(data);
        data.iter().map(|x| (x - mu) * (x - mu)).sum::<f64>() / (data.len() - 1) as f64
    }

    pub fn median(data: &mut [f64]) -> f64 {
        data.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = data.len();
        if n % 2 == 0 {
            (data[n / 2 - 1] + data[n / 2]) / 2.0
        } else {
            data[n / 2]
        }
    }

    /// Percentile (0-100). Linear interpolation.
    pub fn percentile(data: &mut [f64], p: f64) -> f64 {
        data.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = data.len();
        let rank = p / 100.0 * (n - 1) as f64;
        let lo = rank.floor() as usize;
        let hi = rank.ceil() as usize;
        if lo == hi {
            data[lo]
        } else {
            data[lo] + (rank - lo as f64) * (data[hi] - data[lo])
        }
    }

    pub fn min(data: &[f64]) -> f64 {
        data.iter().cloned().fold(f64::INFINITY, f64::min)
    }

    pub fn max(data: &[f64]) -> f64 {
        data.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
    }

    /// Covariance of two variables.
    pub fn covariance(x: &[f64], y: &[f64]) -> f64 {
        assert_eq!(x.len(), y.len());
        let mx = Self::mean(x);
        let my = Self::mean(y);
        x.iter()
            .zip(y)
            .map(|(a, b)| (a - mx) * (b - my))
            .sum::<f64>()
            / x.len() as f64
    }

    /// Pearson correlation coefficient.
    pub fn correlation(x: &[f64], y: &[f64]) -> f64 {
        Self::covariance(x, y) / (Self::std_dev(x) * Self::std_dev(y))
    }
}

/// xoshiro256** PRNG — extremely fast, high quality, 256-bit state.
pub struct Rng {
    state: [u64; 4],
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        // SplitMix64 to initialize state from a single seed
        let mut s = seed;
        let mut state = [0u64; 4];
        for slot in state.iter_mut() {
            s = s.wrapping_add(0x9e3779b97f4a7c15);
            let mut z = s;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
            *slot = z ^ (z >> 31);
        }
        Self { state }
    }

    /// Next u64.
    pub fn next_u64(&mut self) -> u64 {
        let result = self.state[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.state[1] << 17;
        self.state[2] ^= self.state[0];
        self.state[3] ^= self.state[1];
        self.state[1] ^= self.state[2];
        self.state[0] ^= self.state[3];
        self.state[2] ^= t;
        self.state[3] = self.state[3].rotate_left(45);
        result
    }

    /// Uniform f64 in [0, 1).
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Uniform f64 in [lo, hi).
    pub fn uniform(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next_f64()
    }

    /// Normal distribution via Box-Muller transform.
    pub fn normal(&mut self, mean: f64, std_dev: f64) -> f64 {
        let u1 = self.next_f64();
        let u2 = self.next_f64();
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        mean + std_dev * z
    }

    /// Exponential distribution.
    pub fn exponential(&mut self, lambda: f64) -> f64 {
        -self.next_f64().ln() / lambda
    }
}

/// Welch's t-test for two independent samples.
/// Returns (t-statistic, degrees of freedom).
pub fn welch_t_test(x: &[f64], y: &[f64]) -> (f64, f64) {
    let nx = x.len() as f64;
    let ny = y.len() as f64;
    let mx = Stats::mean(x);
    let my = Stats::mean(y);
    let vx = Stats::sample_variance(x);
    let vy = Stats::sample_variance(y);

    let se = (vx / nx + vy / ny).sqrt();
    let t = (mx - my) / se;

    // Welch-Satterthwaite degrees of freedom
    let num = (vx / nx + vy / ny).powi(2);
    let den = (vx / nx).powi(2) / (nx - 1.0) + (vy / ny).powi(2) / (ny - 1.0);
    let df = num / den;

    (t, df)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mean() {
        assert_eq!(Stats::mean(&[1.0, 2.0, 3.0, 4.0, 5.0]), 3.0);
    }

    #[test]
    fn test_variance() {
        assert_eq!(
            Stats::variance(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]),
            4.0
        );
    }

    #[test]
    fn test_std_dev() {
        assert_eq!(
            Stats::std_dev(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]),
            2.0
        );
    }

    #[test]
    fn test_median_odd() {
        assert_eq!(Stats::median(&mut [3.0, 1.0, 2.0]), 2.0);
    }

    #[test]
    fn test_median_even() {
        assert_eq!(Stats::median(&mut [4.0, 1.0, 3.0, 2.0]), 2.5);
    }

    #[test]
    fn test_percentile() {
        assert_eq!(
            Stats::percentile(&mut [10.0, 20.0, 30.0, 40.0, 50.0], 50.0),
            30.0
        );
    }

    #[test]
    fn test_correlation() {
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        let y = [2.0, 4.0, 6.0, 8.0, 10.0];
        assert!((Stats::correlation(&x, &y) - 1.0).abs() < 1e-10); // perfect correlation
    }

    #[test]
    fn test_rng_range() {
        let mut rng = Rng::new(42);
        for _ in 0..100 {
            let v = rng.next_f64();
            assert!(v >= 0.0 && v < 1.0);
        }
    }

    #[test]
    fn test_rng_uniform() {
        let mut rng = Rng::new(123);
        for _ in 0..100 {
            let v = rng.uniform(5.0, 10.0);
            assert!(v >= 5.0 && v < 10.0);
        }
    }

    #[test]
    fn test_rng_normal_mean() {
        let mut rng = Rng::new(42);
        let samples: Vec<f64> = (0..10000).map(|_| rng.normal(0.0, 1.0)).collect();
        let mean = Stats::mean(&samples);
        assert!(mean.abs() < 0.05); // should be near 0
    }

    #[test]
    fn test_welch_t_test() {
        let x = [5.0, 5.1, 4.9, 5.0, 5.2];
        let y = [10.0, 10.1, 9.9, 10.0, 10.2];
        let (t, _df) = welch_t_test(&x, &y);
        assert!(t.abs() > 10.0); // very different means → large t
    }
}
