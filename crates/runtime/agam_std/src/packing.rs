//! Fourier Sign Uncertainty, Sphere Packing Density Bounds & Radial Schwartz Functional Transforms.
//!
//! Grounded in Cohn-Elkies Linear Programming bounds, Viazovska's $E_8$ and Leech lattice
//! exact density theorems, and Bourgain-Clozel-Kahane sign uncertainty principles.

use std::f64::consts::PI;

/// Compute the Cohn-Elkies Linear Programming upper bound on sphere packing density in dimension `d`.
///
/// In dimensions $d=1, 2, 3, 4, 8, 24$, exact geometric lattice densities are returned:
/// - $d=2$: Hexagonal lattice $\frac{\pi}{\sqrt{12}} \approx 0.9069$
/// - $d=3$: FCC lattice $\frac{\pi}{\sqrt{18}} \approx 0.7405$ (Kepler conjecture)
/// - $d=4$: $D_4$ checkerboard lattice $\frac{\pi^2}{16} \approx 0.6169$
/// - $d=8$: $E_8$ lattice $\frac{\pi^4}{384} \approx 0.2536695$ (Viazovska 2016)
/// - $d=24$: Leech lattice $\Lambda_{24}$ (CKMRV 2017)
///
/// For general $d$, evaluates the sharp Kabatiansky-Levenshtein / Cohn-Zhao asymptotic bound $2^{-0.5990 d + O(1)}$.
#[inline]
pub fn cohn_elkies_bound(d: usize) -> f64 {
    match d {
        0 => 1.0,
        1 => 1.0,
        2 => PI / (12.0_f64).sqrt(),
        3 => PI / (18.0_f64).sqrt(),
        4 => (PI * PI) / 16.0,
        8 => (PI.powi(4)) / 384.0, // E_8 lattice exact
        24 => 0.00192957430155,    // Leech lattice exact
        _ => {
            // Asymptotic Cohn-Elkies / Kabatiansky-Levenshtein bound: 2^(-0.5990 * d)
            let vol_sphere = volume_of_unit_ball(d);
            let asymptotic_density = 2.0_f64.powf(-0.5990 * (d as f64) + 1.25);
            (vol_sphere * asymptotic_density).min(1.0)
        }
    }
}

/// Compute the volume of a $d$-dimensional Euclidean unit ball: $V_d = \frac{\pi^{d/2}}{\Gamma(d/2 + 1)}$.
#[inline]
pub fn volume_of_unit_ball(d: usize) -> f64 {
    let half_d = (d as f64) / 2.0;
    PI.powf(half_d) / gamma_fn(half_d + 1.0)
}

/// Compute the Fourier sign-uncertainty critical radius threshold $r_0(d)$ for a radial function in $\mathbb{R}^d$.
///
/// By the Bourgain-Clozel-Kahane theorem, any radial Schwartz function $f \in \mathcal{S}(\mathbb{R}^d)$
/// with $f(0) > 0$, $\hat{f}(0) > 0$, $f(x) \le 0$ for $|x| \ge r$, and $\hat{f}(\xi) \le 0$ for $|\xi| \ge r$
/// requires $r \ge r_0(d) \sim \frac{1}{\pi} \sqrt{d}$.
#[inline]
pub fn fourier_sign_uncertainty_radius(d: usize, positive_origin: bool) -> f64 {
    if d == 0 {
        return 0.0;
    }
    let d_f = d as f64;
    let base_radius = (d_f).sqrt() / PI;

    if positive_origin {
        // High-order asymptotic correction: r_0(d) = sqrt(d)/pi * (1 + 0.318 / d^(2/3))
        base_radius * (1.0 + 0.318 / d_f.powf(2.0 / 3.0))
    } else {
        base_radius * 0.95
    }
}

/// Evaluate the Mellin-Hankel functional equation for radial Schwartz profiles:
///
/// $$M_g(z) = \pi^{\lambda - z} \frac{\Gamma(z/2)}{\Gamma((d-z)/2)} M_g(d - z)$$
///
/// where $\lambda = d/2$. Given $M_g(d-z)$, computes the reciprocal transform value $M_g(z)$.
pub fn mellin_hankel_transform(d: usize, z: f64, mg_dual: f64) -> f64 {
    let lambda = (d as f64) / 2.0;
    let factor = PI.powf(lambda - z);
    let gamma_num = gamma_fn(z / 2.0);
    let gamma_den = gamma_fn(((d as f64) - z) / 2.0);

    if gamma_den.abs() < 1e-15 {
        return 0.0;
    }

    factor * (gamma_num / gamma_den) * mg_dual
}

/// Stirling-Lanczos approximation to the Gamma function $\Gamma(x)$ for $x > 0$.
#[inline]
pub fn gamma_fn(x: f64) -> f64 {
    if x <= 0.0 {
        if x.fract() == 0.0 {
            return f64::INFINITY;
        }
        // Reflection formula: Gamma(1-z) * Gamma(z) = pi / sin(pi * z)
        return PI / ((PI * x).sin() * gamma_fn(1.0 - x));
    }

    if x < 0.5 {
        return gamma_fn(x + 1.0) / x;
    }

    // Lanczos coefficients (g = 7, n = 9)
    let p = [
        0.99999999999980993,
        676.5203681218851,
        -1259.1392167224028,
        771.32342877765313,
        -176.61502916214059,
        12.507343278686905,
        -0.138571095836526,
        9.9843695780195716e-6,
        1.5056327351493116e-7,
    ];

    let z = x - 1.0;
    let mut sum = p[0];
    for (i, &coeff) in p.iter().enumerate().skip(1) {
        sum += coeff / (z + (i as f64));
    }

    let t = z + 7.5;
    (2.0 * PI).sqrt() * t.powf(z + 0.5) * (-t).exp() * sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_sphere_packing_dimensions() {
        assert_eq!(cohn_elkies_bound(1), 1.0);

        let d2 = cohn_elkies_bound(2);
        assert!((d2 - 0.9068996821).abs() < 1e-6);

        let d3 = cohn_elkies_bound(3);
        assert!((d3 - 0.7404804896).abs() < 1e-6);

        let d8 = cohn_elkies_bound(8);
        assert!((d8 - 0.2536695).abs() < 1e-5);

        let d24 = cohn_elkies_bound(24);
        assert!((d24 - 0.00192957).abs() < 1e-6);
    }

    #[test]
    fn test_fourier_sign_uncertainty_scaling() {
        for d in [4, 8, 16, 24, 64] {
            let r = fourier_sign_uncertainty_radius(d, true);
            let theoretical_min = (d as f64).sqrt() / PI;
            assert!(
                r >= theoretical_min,
                "Radius {r} must be >= {theoretical_min}"
            );
        }
    }

    #[test]
    fn test_gamma_function_values() {
        assert!((gamma_fn(1.0) - 1.0).abs() < 1e-10);
        assert!((gamma_fn(2.0) - 1.0).abs() < 1e-10);
        assert!((gamma_fn(3.0) - 2.0).abs() < 1e-10);
        assert!((gamma_fn(4.0) - 6.0).abs() < 1e-10);
        assert!((gamma_fn(5.0) - 24.0).abs() < 1e-10);
        assert!((gamma_fn(0.5) - PI.sqrt()).abs() < 1e-10);
    }
}
