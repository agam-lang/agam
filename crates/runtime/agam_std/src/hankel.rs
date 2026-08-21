//! Hankel Moment Systems & Algebraic Spectral Root Inversion.
//!
//! Implements Prony-Hankel moment inversion algorithms to recover algebraic root sets
//! and weights from power-sum moments $m_k = \sum_{i=1}^N w_i x_i^k$.
//! Used in Reed-Solomon decoding, Gaussian quadrature, and sparse Fourier inversion.

/// Errors that can occur during Hankel moment inversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HankelError {
    InsufficientMoments { required: usize, provided: usize },
    SingularMatrix,
    RootFindingFailed,
    DimensionMismatch,
}

impl std::fmt::Display for HankelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HankelError::InsufficientMoments { required, provided } => {
                write!(
                    f,
                    "Insufficient moments: required {required}, provided {provided}"
                )
            }
            HankelError::SingularMatrix => {
                write!(f, "Hankel moment matrix is singular (rank deficient)")
            }
            HankelError::RootFindingFailed => {
                write!(f, "Failed to find roots of the Prony polynomial")
            }
            HankelError::DimensionMismatch => {
                write!(f, "Matrix and vector dimensions do not match")
            }
        }
    }
}

impl std::error::Error for HankelError {}

/// A symmetric Hankel matrix where entry $H_{i,j} = m_{i+j}$.
#[derive(Debug, Clone, PartialEq)]
pub struct HankelMatrix {
    pub size: usize,
    pub moments: Vec<f64>,
}

impl HankelMatrix {
    /// Create a new Hankel matrix of size $N \times N$ from at least $2N-1$ moments.
    pub fn from_moments(size: usize, moments: &[f64]) -> Result<Self, HankelError> {
        let required = 2 * size - 1;
        if moments.len() < required {
            return Err(HankelError::InsufficientMoments {
                required,
                provided: moments.len(),
            });
        }
        Ok(Self {
            size,
            moments: moments[..required].to_vec(),
        })
    }

    /// Access matrix element $H_{i,j} = m_{i+j}$.
    #[inline]
    pub fn get(&self, row: usize, col: usize) -> f64 {
        self.moments[row + col]
    }

    /// Solve linear system $H x = b$ using Gaussian elimination with partial pivoting.
    pub fn solve(&self, rhs: &[f64]) -> Result<Vec<f64>, HankelError> {
        let n = self.size;
        if rhs.len() != n {
            return Err(HankelError::DimensionMismatch);
        }

        // Build augmented matrix [H | b]
        let mut aug = vec![vec![0.0; n + 1]; n];
        for i in 0..n {
            for j in 0..n {
                aug[i][j] = self.get(i, j);
            }
            aug[i][n] = rhs[i];
        }

        // Forward elimination with partial pivoting
        for col in 0..n {
            let mut max_row = col;
            let mut max_val = aug[col][col].abs();
            for row in (col + 1)..n {
                if aug[row][col].abs() > max_val {
                    max_val = aug[row][col].abs();
                    max_row = row;
                }
            }

            if max_val < 1e-13 {
                return Err(HankelError::SingularMatrix);
            }

            aug.swap(col, max_row);

            let pivot = aug[col][col];
            for row in (col + 1)..n {
                let factor = aug[row][col] / pivot;
                for c in col..=n {
                    aug[row][c] -= factor * aug[col][c];
                }
            }
        }

        // Back substitution
        let mut x = vec![0.0; n];
        for i in (0..n).rev() {
            let mut sum = aug[i][n];
            for j in (i + 1)..n {
                sum -= aug[i][j] * x[j];
            }
            x[i] = sum / aug[i][i];
        }

        Ok(x)
    }

    /// Compute the determinant of the Hankel matrix.
    pub fn determinant(&self) -> f64 {
        let n = self.size;
        let mut mat = vec![vec![0.0; n]; n];
        for i in 0..n {
            for j in 0..n {
                mat[i][j] = self.get(i, j);
            }
        }

        let mut det = 1.0;
        for col in 0..n {
            let mut max_row = col;
            let mut max_val = mat[col][col].abs();
            for row in (col + 1)..n {
                if mat[row][col].abs() > max_val {
                    max_val = mat[row][col].abs();
                    max_row = row;
                }
            }

            if max_val < 1e-15 {
                return 0.0;
            }

            if max_row != col {
                mat.swap(col, max_row);
                det = -det;
            }

            let pivot = mat[col][col];
            det *= pivot;

            for row in (col + 1)..n {
                let factor = mat[row][col] / pivot;
                for c in col..n {
                    mat[row][c] -= factor * mat[col][c];
                }
            }
        }

        det
    }
}

/// Solve the Hankel moment recovery problem.
///
/// Given $2K$ power sum moments $m_0, m_1, \dots, m_{2K-1}$ where $m_k = \sum_{i=1}^K w_i x_i^k$,
/// solves for the exact underlying algebraic root locations $\{x_1, \dots, x_K\}$.
pub fn solve_hankel_system(moments: &[f64]) -> Result<Vec<f64>, HankelError> {
    let k = moments.len() / 2;
    if k == 0 {
        return Ok(Vec::new());
    }

    let h = HankelMatrix::from_moments(k, moments)?;
    let mut rhs = vec![0.0; k];
    for i in 0..k {
        rhs[i] = -moments[k + i];
    }

    // Solve H * c = -rhs for polynomial coefficients c_0, c_1, ..., c_{k-1}
    let coeffs = h.solve(&rhs)?;

    // Monic polynomial: P(x) = x^k + c_{k-1} x^{k-1} + ... + c_1 x + c_0
    let mut poly = vec![0.0; k + 1];
    for i in 0..k {
        poly[i] = coeffs[i];
    }
    poly[k] = 1.0;

    // Find roots of the polynomial using Aberth-Ehrlich / Durand-Kerner iteration
    let roots = find_polynomial_roots(&poly)?;
    Ok(roots)
}

/// Find all real roots of a monic polynomial using Durand-Kerner iteration.
fn find_polynomial_roots(poly: &[f64]) -> Result<Vec<f64>, HankelError> {
    let degree = poly.len() - 1;
    if degree == 1 {
        return Ok(vec![-poly[0] / poly[1]]);
    }

    // Initial root approximations on complex circle
    let mut real_parts = vec![0.0; degree];
    let mut imag_parts = vec![0.0; degree];
    let radius = 1.0
        + poly
            .iter()
            .take(degree)
            .map(|c| c.abs())
            .fold(0.0_f64, f64::max);

    for i in 0..degree {
        let theta = (2.0 * std::f64::consts::PI * (i as f64) + 0.5) / (degree as f64);
        real_parts[i] = radius * theta.cos();
        imag_parts[i] = radius * theta.sin();
    }

    // Iterate
    for _ in 0..200 {
        let mut max_diff = 0.0;
        for i in 0..degree {
            let (p_re, p_im) = eval_complex_poly(poly, real_parts[i], imag_parts[i]);

            let mut prod_re = 1.0;
            let mut prod_im = 0.0;
            for j in 0..degree {
                if i != j {
                    let diff_re = real_parts[i] - real_parts[j];
                    let diff_im = imag_parts[i] - imag_parts[j];
                    let (n_re, n_im) = complex_mul(prod_re, prod_im, diff_re, diff_im);
                    prod_re = n_re;
                    prod_im = n_im;
                }
            }

            let (step_re, step_im) = complex_div(p_re, p_im, prod_re, prod_im);
            real_parts[i] -= step_re;
            imag_parts[i] -= step_im;

            let diff = step_re.hypot(step_im);
            if diff > max_diff {
                max_diff = diff;
            }
        }

        if max_diff < 1e-12 {
            break;
        }
    }

    // Extract roots, sort for deterministic order
    let mut roots: Vec<f64> = real_parts;
    roots.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Ok(roots)
}

#[inline]
fn eval_complex_poly(poly: &[f64], z_re: f64, z_im: f64) -> (f64, f64) {
    let mut val_re = *poly.last().unwrap_or(&0.0);
    let mut val_im = 0.0;

    for &coeff in poly.iter().rev().skip(1) {
        let (m_re, m_im) = complex_mul(val_re, val_im, z_re, z_im);
        val_re = m_re + coeff;
        val_im = m_im;
    }

    (val_re, val_im)
}

#[inline]
fn complex_mul(a_re: f64, a_im: f64, b_re: f64, b_im: f64) -> (f64, f64) {
    (a_re * b_re - a_im * b_im, a_re * b_im + a_im * b_re)
}

#[inline]
fn complex_div(a_re: f64, a_im: f64, b_re: f64, b_im: f64) -> (f64, f64) {
    let denom = b_re * b_re + b_im * b_im;
    if denom < 1e-30 {
        return (0.0, 0.0);
    }
    (
        (a_re * b_re + a_im * b_im) / denom,
        (a_im * b_re - a_re * b_im) / denom,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hankel_determinant_and_solve() {
        // Moments of x1=1, x2=3 with weights w1=1, w2=1
        // m_0 = 1 + 1 = 2
        // m_1 = 1^1 + 3^1 = 4
        // m_2 = 1^2 + 3^2 = 10
        // m_3 = 1^3 + 3^3 = 28
        let moments = vec![2.0, 4.0, 10.0, 28.0];
        let h = HankelMatrix::from_moments(2, &moments).expect("Hankel creation");

        // H = [[2, 4], [4, 10]], det = 20 - 16 = 4
        assert!((h.determinant() - 4.0).abs() < 1e-10);

        let roots = solve_hankel_system(&moments).expect("Hankel solve");
        assert_eq!(roots.len(), 2);
        assert!(
            (roots[0] - 1.0).abs() < 1e-4,
            "Expected root 1.0, got {}",
            roots[0]
        );
        assert!(
            (roots[1] - 3.0).abs() < 1e-4,
            "Expected root 3.0, got {}",
            roots[1]
        );
    }

    #[test]
    fn test_hankel_three_roots() {
        // Roots x1 = -2, x2 = 1, x3 = 5 with equal weights 1
        let roots_true: [f64; 3] = [-2.0, 1.0, 5.0];
        let mut moments = vec![0.0; 6];
        for k in 0..6 {
            moments[k] = roots_true.iter().map(|&x: &f64| x.powi(k as i32)).sum();
        }

        let recovered = solve_hankel_system(&moments).expect("Solve 3 roots");
        assert_eq!(recovered.len(), 3);
        assert!((recovered[0] - (-2.0)).abs() < 1e-3);
        assert!((recovered[1] - 1.0).abs() < 1e-3);
        assert!((recovered[2] - 5.0).abs() < 1e-3);
    }
}
