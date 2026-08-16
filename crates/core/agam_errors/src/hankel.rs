//! Hankel Moment Matrix Root Solvers for Algebraic Type Constraint Solvers.
//!
//! Implements Hankel determinant calculation ($\Delta_h = \det(\mu_{i+j})$) and
//! moment-based algebraic pole/root extraction for error recovery and constraint resolution.

/// A square Hankel matrix where entry (i, j) depends only on (i + j).
#[derive(Debug, Clone, PartialEq)]
pub struct HankelMatrix {
    pub order: usize,
    pub data: Vec<Vec<f64>>,
}

impl HankelMatrix {
    /// Construct an n x n Hankel matrix from a moment sequence $\mu_0, \dots, \mu_{2n-2}$.
    pub fn from_moments(moments: &[f64], order: usize) -> Result<Self, String> {
        let required = 2 * order - 1;
        if moments.len() < required {
            return Err(format!(
                "insufficient moments for order {order}: need at least {required}, got {}",
                moments.len()
            ));
        }

        let mut data = Vec::with_capacity(order);
        for i in 0..order {
            let mut row = Vec::with_capacity(order);
            for j in 0..order {
                row.push(moments[i + j]);
            }
            data.push(row);
        }

        Ok(Self { order, data })
    }

    /// Calculate the determinant of the Hankel matrix using Gaussian elimination.
    #[allow(clippy::needless_range_loop)]
    pub fn determinant(&self) -> f64 {
        let n = self.order;
        if n == 0 {
            return 1.0;
        }
        if n == 1 {
            return self.data[0][0];
        }
        if n == 2 {
            return self.data[0][0] * self.data[1][1] - self.data[0][1] * self.data[1][0];
        }

        let mut a = self.data.clone();
        let mut det = 1.0;

        for i in 0..n {
            // Find pivot
            let mut max_row = i;
            let mut max_val = a[i][i].abs();
            for k in (i + 1)..n {
                if a[k][i].abs() > max_val {
                    max_val = a[k][i].abs();
                    max_row = k;
                }
            }

            if max_val < 1e-12 {
                return 0.0;
            }

            if max_row != i {
                a.swap(i, max_row);
                det = -det;
            }

            let pivot = a[i][i];
            det *= pivot;

            for k in (i + 1)..n {
                let factor = a[k][i] / pivot;
                for j in i..n {
                    let sub = factor * a[i][j];
                    a[k][j] -= sub;
                }
            }
        }

        det
    }

    /// Solves the underlying algebraic poles for a 2nd-order moment sequence.
    ///
    /// For moments $\mu_0, \mu_1, \mu_2, \mu_3$, finds roots of $c_0 + c_1 z + z^2 = 0$
    /// where $H_2 \cdot [c_0, c_1]^T = -[\mu_2, \mu_3]^T$.
    pub fn solve_order2_poles(moments: &[f64]) -> Option<(f64, f64)> {
        if moments.len() < 4 {
            return None;
        }
        let h = HankelMatrix::from_moments(moments, 2).ok()?;
        let det = h.determinant();
        if det.abs() < 1e-12 {
            return None;
        }

        // Solve 2x2 system:
        // [mu0 mu1] [c0] = [-mu2]
        // [mu1 mu2] [c1]   [-mu3]
        let mu0 = moments[0];
        let mu1 = moments[1];
        let mu2 = moments[2];
        let mu3 = moments[3];

        let c0 = (-mu2 * mu2 - (-mu3 * mu1)) / det;
        let c1 = (mu0 * (-mu3) - mu1 * (-mu2)) / det;

        // Solve quadratic equation: z^2 + c1 * z + c0 = 0
        let discriminant = c1 * c1 - 4.0 * c0;
        if discriminant < 0.0 {
            return None;
        }

        let sqrt_d = discriminant.sqrt();
        let r1 = (-c1 + sqrt_d) / 2.0;
        let r2 = (-c1 - sqrt_d) / 2.0;

        Some((r1, r2))
    }
}

/// Solves and reconstructs algebraic roots from moment samples.
pub fn solve_hankel_determinant(moments: &[f64], order: usize) -> Option<f64> {
    let matrix = HankelMatrix::from_moments(moments, order).ok()?;
    Some(matrix.determinant())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hankel_matrix_determinant_order2() {
        // Moments: [1.0, 2.0, 5.0] -> [[1.0, 2.0], [2.0, 5.0]] -> det = 1*5 - 2*2 = 1.0
        let moments = vec![1.0, 2.0, 5.0];
        let h = HankelMatrix::from_moments(&moments, 2).unwrap();
        let det = h.determinant();
        assert!((det - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_hankel_matrix_determinant_order3() {
        // Geometric progression: moments = [1, 2, 4, 8, 16] -> rank 1, det = 0
        let moments = vec![1.0, 2.0, 4.0, 8.0, 16.0];
        let h = HankelMatrix::from_moments(&moments, 3).unwrap();
        let det = h.determinant();
        assert!(det.abs() < 1e-9);
    }

    #[test]
    fn test_hankel_order2_pole_solver() {
        // Two poles at z1 = 2.0, z2 = 3.0 with weights w1 = 1.0, w2 = 1.0:
        // mu_k = 1.0 * 2^k + 1.0 * 3^k
        // mu0 = 1 + 1 = 2
        // mu1 = 2 + 3 = 5
        // mu2 = 4 + 9 = 13
        // mu3 = 8 + 27 = 35
        let moments = vec![2.0, 5.0, 13.0, 35.0];
        let (r1, r2) = HankelMatrix::solve_order2_poles(&moments).unwrap();
        assert!(
            ((r1 - 3.0).abs() < 1e-6 && (r2 - 2.0).abs() < 1e-6)
                || ((r1 - 2.0).abs() < 1e-6 && (r2 - 3.0).abs() < 1e-6)
        );
    }
}
