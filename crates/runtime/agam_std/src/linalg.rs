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

    /// Matrix-matrix multiply (C = A * B).
    pub fn matmul(&self, other: &Matrix) -> Option<Matrix> {
        if self.cols != other.rows {
            return None;
        }
        let mut out = Matrix::zeros(self.rows, other.cols);
        for i in 0..self.rows {
            for k in 0..self.cols {
                let a_ik = self.get(i, k);
                for j in 0..other.cols {
                    let current = out.get(i, j);
                    out.set(i, j, current + a_ik * other.get(k, j));
                }
            }
        }
        Some(out)
    }

    /// Matrix-vector multiply (Ax = b). Returns vector b.
    pub fn matvec(&self, x: &[f64]) -> Vec<f64> {
        assert_eq!(self.cols, x.len());
        let mut result = vec![0.0; self.rows];
        for (i, value) in result.iter_mut().enumerate().take(self.rows) {
            let row_start = i * self.cols;
            let mut sum = 0.0;
            for (j, input) in x.iter().enumerate().take(self.cols) {
                sum += self.data[row_start + j] * input;
            }
            *value = sum;
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

    /// Compute the matrix permanent via Glynn's formula in O(n 2^(n-1)) operations.
    pub fn permanent(&self) -> f64 {
        assert_eq!(self.rows, self.cols, "permanent requires square matrix");
        glynn_permanent(&self.data, self.rows)
    }
}

/// Compute the matrix permanent of an n x n row-major matrix using Ryser's inclusion-exclusion formula:
///
/// $$\operatorname{perm}(A) = (-1)^n \sum_{S \subseteq \{0,\dots,n-1\}} (-1)^{|S|} \prod_{i=0}^{n-1} \sum_{j \in S} A_{i,j}$$
pub fn ryser_permanent(data: &[f64], n: usize) -> f64 {
    if n == 0 {
        return 1.0;
    }
    if n == 1 {
        return data[0];
    }

    let num_subsets = 1usize << n;
    let mut perm_sum = 0.0;

    for mask in 0..num_subsets {
        let size = mask.count_ones();
        let sign = if size % 2 == 0 { 1.0 } else { -1.0 };

        let mut row_prod = 1.0;
        for i in 0..n {
            let mut col_sum = 0.0;
            for j in 0..n {
                if (mask & (1 << j)) != 0 {
                    col_sum += data[i * n + j];
                }
            }
            row_prod *= col_sum;
        }

        perm_sum += sign * row_prod;
    }

    let global_sign = if n.is_multiple_of(2) { 1.0 } else { -1.0 };
    global_sign * perm_sum
}

/// Compute the matrix permanent of an n x n row-major matrix using Glynn's balanced formula:
///
/// $$\operatorname{perm}(A) = \frac{1}{2^{n-1}} \sum_{\delta \in \{-1, 1\}^n, \delta_0 = 1} \left( \prod_{k=0}^{n-1} \delta_k \right) \prod_{i=0}^{n-1} \left( \sum_{j=0}^{n-1} \delta_j A_{i,j} \right)$$
pub fn glynn_permanent(data: &[f64], n: usize) -> f64 {
    if n == 0 {
        return 1.0;
    }
    if n == 1 {
        return data[0];
    }

    let num_patterns = 1usize << (n - 1);
    let mut total_sum = 0.0;

    for mask in 0..num_patterns {
        let mut delta = vec![1.0; n];
        let mut prod_delta = 1.0;

        for (j, d) in delta.iter_mut().enumerate().skip(1) {
            if (mask & (1 << (j - 1))) != 0 {
                *d = -1.0;
                prod_delta = -prod_delta;
            }
        }

        let mut row_prod = 1.0;
        for i in 0..n {
            let mut col_sum = 0.0;
            for j in 0..n {
                col_sum += delta[j] * data[i * n + j];
            }
            row_prod *= col_sum;
        }

        total_sum += prod_delta * row_prod;
    }

    total_sum / (2.0_f64.powi((n - 1) as i32))
}

/// Compute vector dot product: sum(a_i * b_i).
pub fn dot(a: &[f64], b: &[f64]) -> Option<f64> {
    if a.len() != b.len() {
        return None;
    }
    Some(a.iter().zip(b).map(|(x, y)| x * y).sum())
}

/// Compute matrix multiplication: C = A * B.
pub fn matmul(a: &Matrix, b: &Matrix) -> Option<Matrix> {
    a.matmul(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dot_and_matmul() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        assert_eq!(dot(&a, &b), Some(32.0)); // 1*4 + 2*5 + 3*6 = 32

        let m1 = Matrix::new(2, 3, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let m2 = Matrix::new(3, 2, vec![7.0, 8.0, 9.0, 1.0, 2.0, 3.0]);
        let res = matmul(&m1, &m2);
        assert!(res.is_some());
        if let Some(c) = res {
            assert_eq!(c.rows, 2);
            assert_eq!(c.cols, 2);
            assert_eq!(c.get(0, 0), 31.0); // 1*7 + 2*9 + 3*2 = 31
            assert_eq!(c.get(0, 1), 19.0); // 1*8 + 2*1 + 3*3 = 19
        }
    }



    #[test]
    fn test_inverse_2x2() {
        let m = Matrix::new(2, 2, vec![4.0, 7.0, 2.0, 6.0]);
        let inv_opt = m.inverse();
        assert!(inv_opt.is_some());
        if let Some(inv) = inv_opt {
            let prod = matmul(&m, &inv);
            assert!(prod.is_some());
            if let Some(p) = prod {
                assert!((p.get(0, 0) - 1.0).abs() < 1e-10);
                assert!((p.get(1, 1) - 1.0).abs() < 1e-10);
            }
        }
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
