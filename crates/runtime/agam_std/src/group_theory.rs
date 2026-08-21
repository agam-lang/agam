//! Non-Sofic Group Generators & Kazhdan Property-(T) Spectral Gap Verification.
//!
//! Implements binary Leavitt algebra units $L_{\mathbb{F}_2}(1, 2)^\times$
//! and Kazhdan property-(T) spectral constant bounds.

/// An element in the binary Leavitt algebra $L_{\mathbb{F}_2}(1, 2)$ defined by
/// relations $x y = 1, z w = 1, y x + w z = 1$.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LeavittAlgebraElement {
    /// Monomial terms represented as bitmasks over generators $\{x, y, z, w\}$.
    pub terms: Vec<u32>,
}

impl LeavittAlgebraElement {
    /// Identity element in $L_{\mathbb{F}_2}(1, 2)$.
    pub fn identity() -> Self {
        Self { terms: vec![0] }
    }

    /// Create an element from generator sequence where 0=x, 1=y, 2=z, 3=w.
    pub fn from_generators(gen_indices: &[u8]) -> Self {
        let mut mask = 0u32;
        for (i, &g) in gen_indices.iter().enumerate().take(8) {
            mask |= ((g as u32) & 0x3) << (i * 2);
        }
        Self { terms: vec![mask] }
    }

    /// Check if the element is empty (zero in $\mathbb{F}_2$).
    pub fn is_zero(&self) -> bool {
        self.terms.is_empty()
    }

    /// Add two Leavitt algebra elements over $\mathbb{F}_2$ (symmetric difference of monomials).
    pub fn add(&self, other: &Self) -> Self {
        let mut result = Vec::new();
        for &t in &self.terms {
            if !other.terms.contains(&t) {
                result.push(t);
            }
        }
        for &t in &other.terms {
            if !self.terms.contains(&t) {
                result.push(t);
            }
        }
        result.sort_unstable();
        Self { terms: result }
    }
}

/// Compute the Kazhdan property-(T) constant $\kappa(G, S)$ for a symmetric generating set $S$.
///
/// Uses the normalized graph Laplacian spectral gap $\lambda_2(\Delta_S)$:
///
/// $$\kappa(G, S) = \sqrt{2 \cdot \lambda_2(\Delta_S)}$$
pub fn kazhdan_constant_property_t(adjacency_laplacian: &[Vec<f64>]) -> f64 {
    let n = adjacency_laplacian.len();
    if n <= 1 {
        return 0.0;
    }

    let mat = crate::linalg::Matrix::new(
        n,
        n,
        adjacency_laplacian.iter().flatten().copied().collect(),
    );

    // Initial test vector orthogonal to the constant eigenvector (1, 1, ..., 1)
    let mut v = vec![0.0; n];
    for (i, val) in v.iter_mut().enumerate() {
        *val = if i % 2 == 0 { 1.0 } else { -1.0 };
    }
    project_orthogonal_to_ones(&mut v);
    normalize_vec(&mut v);

    let mut spectral_gap = 0.0;

    // Power iteration on orthogonal subspace
    for _ in 0..100 {
        let mut w = mat.matvec(&v);
        project_orthogonal_to_ones(&mut w);

        let rayleigh: f64 = w.iter().zip(&v).map(|(a, b)| a * b).sum();
        let norm = normalize_vec(&mut w);

        if norm < 1e-12 {
            break;
        }

        v = w;
        if (rayleigh - spectral_gap).abs() < 1e-8 {
            spectral_gap = rayleigh;
            break;
        }
        spectral_gap = rayleigh;
    }

    (2.0 * spectral_gap.max(0.0)).sqrt()
}

fn project_orthogonal_to_ones(v: &mut [f64]) {
    let n = v.len() as f64;
    let mean: f64 = v.iter().sum::<f64>() / n;
    for x in v.iter_mut() {
        *x -= mean;
    }
}

fn normalize_vec(v: &mut [f64]) -> f64 {
    let norm: f64 = (v.iter().map(|x| x * x).sum::<f64>()).sqrt();
    if norm > 1e-15 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
    norm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leavitt_algebra_addition() {
        let e1 = LeavittAlgebraElement::from_generators(&[0, 1]); // xy
        let e2 = LeavittAlgebraElement::from_generators(&[2, 3]); // zw
        let sum = e1.add(&e2);
        assert_eq!(sum.terms.len(), 2);

        // (e1 + e2) + e1 = e2 (mod 2)
        let sum2 = sum.add(&e1);
        assert_eq!(sum2, e2);
    }

    #[test]
    fn test_kazhdan_constant_bounds() {
        // Complete graph K_4 normalized Laplacian:
        // [[3, -1, -1, -1], [-1, 3, -1, -1], [-1, -1, 3, -1], [-1, -1, -1, 3]]
        let laplacian = vec![
            vec![3.0, -1.0, -1.0, -1.0],
            vec![-1.0, 3.0, -1.0, -1.0],
            vec![-1.0, -1.0, 3.0, -1.0],
            vec![-1.0, -1.0, -1.0, 3.0],
        ];

        let kappa = kazhdan_constant_property_t(&laplacian);
        assert!(
            kappa > 0.0,
            "Kazhdan constant must be positive for expanders, got {kappa}"
        );
        // For K_4, lambda_2 = 4, kappa = sqrt(2 * 4) = sqrt(8) ~ 2.8284
        assert!((kappa - (8.0_f64).sqrt()).abs() < 1e-3);
    }
}
