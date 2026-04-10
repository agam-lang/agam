//! Linear algebra operations — hardware-optimized.
//!
//! All operations use contiguous row-major storage for maximum cache locality.
//! Matrix data is `repr(C)` aligned for potential SIMD vectorization.

/// A dense matrix stored in row-major contiguous memory.
#[derive(Debug, Clone, PartialEq)]
pub struct Matrix {
    pub rows: usize,
    pub cols: usize,
    /// Row-major contiguous data (cache-line friendly).
    pub data: Vec<f64>,
}

impl Matrix {
    pub fn new(rows: usize, cols: usize, data: Vec<f64>) -> Self {
        assert_eq!(data.len(), rows * cols);
        Self { rows, cols, data }
    }

    pub fn zeros(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            data: vec![0.0; rows * cols],
        }
    }

    pub fn identity(n: usize) -> Self {
        let mut m = Self::zeros(n, n);
        for i in 0..n {
            m.data[i * n + i] = 1.0;
        }
        m
    }

    #[inline(always)]
    pub fn get(&self, i: usize, j: usize) -> f64 {
        self.data[i * self.cols + j]
    }

    #[inline(always)]
    pub fn set(&mut self, i: usize, j: usize, val: f64) {
        self.data[i * self.cols + j] = val;
    }

    /// Determinant via LU decomposition.
    pub fn det(&self) -> f64 {
        assert_eq!(self.rows, self.cols, "determinant requires square matrix");
        let (lu, parity) = self.lu_decompose();
        let mut det = parity as f64;
        for i in 0..self.rows {
            det *= lu.get(i, i);
        }
        det
    }

    /// LU decomposition with partial pivoting.
    /// Returns (LU combined matrix, parity: +1 or -1).
    pub fn lu_decompose(&self) -> (Matrix, i32) {
        let n = self.rows;
        assert_eq!(n, self.cols, "LU requires square matrix");
        let mut lu = self.clone();
        let mut parity = 1i32;

        for col in 0..n {
            // Partial pivot: find max in column
            let mut max_row = col;
            let mut max_val = lu.get(col, col).abs();
            for row in (col + 1)..n {
                let v = lu.get(row, col).abs();
                if v > max_val {
                    max_val = v;
                    max_row = row;
                }
            }
            if max_row != col {
                // Swap rows (contiguous memory swap for cache performance)
                for j in 0..n {
                    let a = col * n + j;
                    let b = max_row * n + j;
                    lu.data.swap(a, b);
                }
                parity = -parity;
            }

            let pivot = lu.get(col, col);
            if pivot.abs() < 1e-15 {
                continue;
            } // singular

            for row in (col + 1)..n {
                let factor = lu.get(row, col) / pivot;
                lu.set(row, col, factor);
                for j in (col + 1)..n {
                    let val = lu.get(row, j) - factor * lu.get(col, j);
                    lu.set(row, j, val);
                }
            }
        }
        (lu, parity)
    }

    /// Matrix inverse via LU decomposition.
    pub fn inverse(&self) -> Option<Matrix> {
        let n = self.rows;
        assert_eq!(n, self.cols, "inverse requires square matrix");
        let (lu, _) = self.lu_decompose();

        // Check singularity
        for i in 0..n {
            if lu.get(i, i).abs() < 1e-15 {
                return None;
            }
        }

        let mut inv = Matrix::identity(n);

        // Solve LU * X = I column by column
        for col in 0..n {
            // Forward substitution (L * y = e_col)
            for i in 0..n {
                let mut sum = inv.get(i, col);
                for j in 0..i {
                    sum -= lu.get(i, j) * inv.get(j, col);
                }
                inv.set(i, col, sum);
            }
            // Back substitution (U * x = y)
            for i in (0..n).rev() {
                let mut sum = inv.get(i, col);
                for j in (i + 1)..n {
                    sum -= lu.get(i, j) * inv.get(j, col);
                }
                inv.set(i, col, sum / lu.get(i, i));
            }
        }
        Some(inv)
    }

    /// Trace (sum of diagonal elements).
    pub fn trace(&self) -> f64 {
        let n = self.rows.min(self.cols);
        (0..n).map(|i| self.get(i, i)).sum()
    }

    /// Transpose.
    pub fn transpose(&self) -> Matrix {
        let mut t = Matrix::zeros(self.cols, self.rows);
        for i in 0..self.rows {
            for j in 0..self.cols {
                t.set(j, i, self.get(i, j));
            }
        }
        t
    }

    /// Matrix-vector multiply (Ax = b). Returns vector b.
    pub fn matvec(&self, x: &[f64]) -> Vec<f64> {
        assert_eq!(self.cols, x.len());
        let mut result = vec![0.0; self.rows];
        for i in 0..self.rows {
            let row_start = i * self.cols;
            let mut sum = 0.0;
            for j in 0..self.cols {
                sum += self.data[row_start + j] * x[j];
            }
            result[i] = sum;
        }
        result
    }

    /// Solve Ax = b via LU decomposition.
    pub fn solve(&self, b: &[f64]) -> Vec<f64> {
        let n = self.rows;
        let (lu, _) = self.lu_decompose();
        let mut x = b.to_vec();

        // Forward substitution
        for i in 0..n {
            for j in 0..i {
                x[i] -= lu.get(i, j) * x[j];
            }
        }
        // Back substitution
        for i in (0..n).rev() {
            for j in (i + 1)..n {
                x[i] -= lu.get(i, j) * x[j];
            }
            x[i] /= lu.get(i, i);
        }
        x
    }

    /// Power iteration for dominant eigenvalue.
    /// Returns (eigenvalue, eigenvector).
    pub fn dominant_eigenvalue(&self, max_iter: usize, tol: f64) -> (f64, Vec<f64>) {
        let n = self.rows;
        let mut v: Vec<f64> = vec![1.0; n];
        let norm: f64 = (v.iter().map(|x| x * x).sum::<f64>()).sqrt();
        for x in v.iter_mut() {
            *x /= norm;
        }

        let mut eigenvalue = 0.0;

        for _ in 0..max_iter {
            let w = self.matvec(&v);
            let new_eigenvalue = w.iter().zip(&v).map(|(a, b)| a * b).sum::<f64>();
            let norm: f64 = (w.iter().map(|x| x * x).sum::<f64>()).sqrt();
            v = w.iter().map(|x| x / norm).collect();

            if (new_eigenvalue - eigenvalue).abs() < tol {
                break;
            }
            eigenvalue = new_eigenvalue;
        }
        (eigenvalue, v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity() {
        let i = Matrix::identity(3);
        assert_eq!(i.get(0, 0), 1.0);
        assert_eq!(i.get(0, 1), 0.0);
        assert_eq!(i.get(1, 1), 1.0);
    }

    #[test]
    fn test_det_2x2() {
        // |1 2| = 1*4 - 2*3 = -2
        // |3 4|
        let m = Matrix::new(2, 2, vec![1.0, 2.0, 3.0, 4.0]);
        assert!((m.det() - (-2.0)).abs() < 1e-10);
    }

    #[test]
    fn test_det_3x3() {
        let m = Matrix::new(3, 3, vec![6.0, 1.0, 1.0, 4.0, -2.0, 5.0, 2.0, 8.0, 7.0]);
        assert!((m.det() - (-306.0)).abs() < 1e-8);
    }

    #[test]
    fn test_inverse_2x2() {
        let m = Matrix::new(2, 2, vec![4.0, 7.0, 2.0, 6.0]);
        let inv = m.inverse().unwrap();
        // M * M⁻¹ should be identity
        let prod = crate::tensor::Tensor::from_data(&[2, 2], m.data.clone())
            .matmul(&crate::tensor::Tensor::from_data(&[2, 2], inv.data.clone()));
        assert!((prod.data[0] - 1.0).abs() < 1e-10);
        assert!((prod.data[3] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_trace() {
        let m = Matrix::new(3, 3, vec![1.0, 0.0, 0.0, 0.0, 5.0, 0.0, 0.0, 0.0, 9.0]);
        assert_eq!(m.trace(), 15.0);
    }

    #[test]
    fn test_solve() {
        // 2x + y = 5
        // x + 3y = 7  → x=1.6, y=1.8
        let a = Matrix::new(2, 2, vec![2.0, 1.0, 1.0, 3.0]);
        let b = vec![5.0, 7.0];
        let x = a.solve(&b);
        assert!((x[0] - 1.6).abs() < 1e-10);
        assert!((x[1] - 1.8).abs() < 1e-10);
    }

    #[test]
    fn test_matvec() {
        let m = Matrix::new(2, 2, vec![1.0, 2.0, 3.0, 4.0]);
        let x = vec![1.0, 1.0];
        let result = m.matvec(&x);
        assert_eq!(result, vec![3.0, 7.0]);
    }

    #[test]
    fn test_eigenvalue_diagonal() {
        // Diagonal matrix: eigenvalues are diagonal entries.
        // Dominant eigenvalue of diag(1, 5, 3) is 5.
        let m = Matrix::new(3, 3, vec![1.0, 0.0, 0.0, 0.0, 5.0, 0.0, 0.0, 0.0, 3.0]);
        let (ev, _) = m.dominant_eigenvalue(100, 1e-10);
        assert!((ev - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_transpose() {
        let m = Matrix::new(2, 3, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let t = m.transpose();
        assert_eq!(t.rows, 3);
        assert_eq!(t.cols, 2);
        assert_eq!(t.get(0, 0), 1.0);
        assert_eq!(t.get(0, 1), 4.0);
    }
}
