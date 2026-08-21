//! Hardware-Aware Sparse Matrix Primitives (CSR, CSC, COO Formats & SpMV/SpMM Kernels).

use serde::{Deserialize, Serialize};

/// Compressed Sparse Row (CSR) matrix format for fast row-slicing and SpMV.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CsrMatrix {
    pub rows: usize,
    pub cols: usize,
    pub values: Vec<f64>,
    pub col_indices: Vec<usize>,
    pub row_offsets: Vec<usize>,
}

impl CsrMatrix {
    /// Construct a new CSR matrix from raw vectors.
    pub fn new(
        rows: usize,
        cols: usize,
        values: Vec<f64>,
        col_indices: Vec<usize>,
        row_offsets: Vec<usize>,
    ) -> Self {
        assert_eq!(row_offsets.len(), rows + 1);
        assert_eq!(values.len(), col_indices.len());
        Self {
            rows,
            cols,
            values,
            col_indices,
            row_offsets,
        }
    }

    /// Convert Coordinate (COO) entries into CSR representation.
    pub fn from_coo(rows: usize, cols: usize, mut entries: Vec<(usize, usize, f64)>) -> Self {
        entries.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

        let mut values = Vec::with_capacity(entries.len());
        let mut col_indices = Vec::with_capacity(entries.len());
        let mut row_offsets = vec![0; rows + 1];

        for (r, c, val) in &entries {
            values.push(*val);
            col_indices.push(*c);
            row_offsets[r + 1] += 1;
        }

        // Compute prefix sum for row_offsets
        for i in 0..rows {
            row_offsets[i + 1] += row_offsets[i];
        }

        Self {
            rows,
            cols,
            values,
            col_indices,
            row_offsets,
        }
    }

    /// Number of non-zero entries (NNZ).
    pub fn nnz(&self) -> usize {
        self.values.len()
    }

    /// Sparsity ratio (0.0 = dense, 1.0 = completely empty).
    pub fn sparsity(&self) -> f64 {
        let total = self.rows * self.cols;
        if total == 0 {
            0.0
        } else {
            1.0 - (self.nnz() as f64 / total as f64)
        }
    }

    /// High-performance Sparse Matrix-Vector Multiplication ($y = A \cdot x$).
    pub fn spmv(&self, x: &[f64]) -> Vec<f64> {
        assert_eq!(
            x.len(),
            self.cols,
            "Vector dimension must match matrix columns"
        );
        let mut y = vec![0.0; self.rows];

        for (i, y_slot) in y.iter_mut().enumerate() {
            let start = self.row_offsets[i];
            let end = self.row_offsets[i + 1];
            let mut sum = 0.0;
            for idx in start..end {
                sum += self.values[idx] * x[self.col_indices[idx]];
            }
            *y_slot = sum;
        }

        y
    }
}

/// Coordinate (COO) list format for dynamic construction.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CooMatrix {
    pub rows: usize,
    pub cols: usize,
    pub entries: Vec<(usize, usize, f64)>,
}

impl CooMatrix {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            entries: Vec::new(),
        }
    }

    pub fn insert(&mut self, row: usize, col: usize, val: f64) {
        assert!(row < self.rows && col < self.cols, "Index out of bounds");
        self.entries.push((row, col, val));
    }

    pub fn to_csr(&self) -> CsrMatrix {
        CsrMatrix::from_coo(self.rows, self.cols, self.entries.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coo_to_csr_and_spmv() {
        // Construct a 3x3 sparse matrix:
        // [ 10.0,  0.0,  20.0 ]
        // [  0.0, 30.0,   0.0 ]
        // [  0.0,  0.0,  40.0 ]
        let mut coo = CooMatrix::new(3, 3);
        coo.insert(0, 0, 10.0);
        coo.insert(0, 2, 20.0);
        coo.insert(1, 1, 30.0);
        coo.insert(2, 2, 40.0);

        let csr = coo.to_csr();
        assert_eq!(csr.nnz(), 4);
        assert_eq!(csr.row_offsets, vec![0, 2, 3, 4]);

        let x = vec![1.0, 2.0, 3.0];
        let y = csr.spmv(&x);

        // y[0] = 10*1 + 20*3 = 70
        // y[1] = 30*2 = 60
        // y[2] = 40*3 = 120
        assert_eq!(y, vec![70.0, 60.0, 120.0]);
    }
}
