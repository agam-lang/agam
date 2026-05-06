//! N-dimensional Tensor type for Agam.
//!
//! Provides a shape-aware, contiguous-memory tensor with support for
//! element-wise operations, broadcasting, and basic linear algebra.
//!
//! This is the foundation for Agam's native AI/ML capabilities.

use agam_runtime::simd::SimdOps;

/// An N-dimensional tensor stored in row-major contiguous memory.
#[derive(Debug, Clone, PartialEq)]
pub struct Tensor {
    /// The shape of the tensor: e.g. [2, 3] for a 2×3 matrix.
    pub shape: Vec<usize>,
    /// Flat data storage in row-major order.
    pub data: Vec<f64>,
}

impl Tensor {
    /// Create a tensor with the given shape, filled with zeros.
    pub fn zeros(shape: &[usize]) -> Self {
        let size: usize = shape.iter().product();
        Self {
            shape: shape.to_vec(),
            data: vec![0.0; size],
        }
    }

    /// Create a tensor with the given shape, filled with ones.
    pub fn ones(shape: &[usize]) -> Self {
        let size: usize = shape.iter().product();
        Self {
            shape: shape.to_vec(),
            data: vec![1.0; size],
        }
    }

    /// Create a tensor from flat data and shape.
    pub fn from_data(shape: &[usize], data: Vec<f64>) -> Self {
        let expected: usize = shape.iter().product();
        assert_eq!(data.len(), expected, "data length must match shape product");
        Self {
            shape: shape.to_vec(),
            data,
        }
    }

    /// Create a scalar tensor.
    pub fn scalar(val: f64) -> Self {
        Self {
            shape: vec![],
            data: vec![val],
        }
    }

    /// Create a 1D tensor (vector).
    pub fn vector(data: Vec<f64>) -> Self {
        let len = data.len();
        Self {
            shape: vec![len],
            data,
        }
    }

    /// Total number of elements.
    pub fn numel(&self) -> usize {
        self.data.len()
    }

    /// Number of dimensions.
    pub fn ndim(&self) -> usize {
        self.shape.len()
    }

    /// Get element at flat index.
    pub fn get_flat(&self, idx: usize) -> f64 {
        self.data[idx]
    }

    /// Set element at flat index.
    pub fn set_flat(&mut self, idx: usize, val: f64) {
        self.data[idx] = val;
    }

    /// Element-wise addition. Shapes must match.
    pub fn add(&self, other: &Tensor) -> Tensor {
        assert_eq!(self.shape, other.shape, "shapes must match for add");
        let mut data = vec![0.0; self.numel()];
        SimdOps::add(&self.data, &other.data, &mut data);
        Tensor {
            shape: self.shape.clone(),
            data,
        }
    }

    /// Element-wise subtraction. Shapes must match.
    pub fn sub(&self, other: &Tensor) -> Tensor {
        assert_eq!(self.shape, other.shape, "shapes must match for sub");
        let mut data = vec![0.0; self.numel()];
        SimdOps::sub(&self.data, &other.data, &mut data);
        Tensor {
            shape: self.shape.clone(),
            data,
        }
    }

    /// Element-wise multiplication (Hadamard). Shapes must match.
    pub fn mul(&self, other: &Tensor) -> Tensor {
        assert_eq!(self.shape, other.shape, "shapes must match for mul");
        let mut data = vec![0.0; self.numel()];
        SimdOps::mul(&self.data, &other.data, &mut data);
        Tensor {
            shape: self.shape.clone(),
            data,
        }
    }

    /// Scalar multiplication.
    pub fn scale(&self, s: f64) -> Tensor {
        let mut data = vec![0.0; self.numel()];
        SimdOps::scale(&self.data, s, &mut data);
        Tensor {
            shape: self.shape.clone(),
            data,
        }
    }

    /// Sum all elements.
    pub fn sum(&self) -> f64 {
        SimdOps::sum(&self.data)
    }

    /// Mean of all elements.
    pub fn mean(&self) -> f64 {
        self.sum() / self.numel() as f64
    }

    /// Dot product (for 1D tensors / vectors).
    pub fn dot(&self, other: &Tensor) -> f64 {
        assert_eq!(self.ndim(), 1, "dot requires 1D tensors");
        assert_eq!(other.ndim(), 1, "dot requires 1D tensors");
        assert_eq!(self.shape[0], other.shape[0], "lengths must match");
        SimdOps::dot(&self.data, &other.data)
    }

    /// Matrix multiplication for 2D tensors.
    /// self: [M, K], other: [K, N] → result: [M, N]
    pub fn matmul(&self, other: &Tensor) -> Tensor {
        assert_eq!(self.ndim(), 2, "matmul requires 2D tensors");
        assert_eq!(other.ndim(), 2, "matmul requires 2D tensors");
        let m = self.shape[0];
        let k = self.shape[1];
        assert_eq!(k, other.shape[0], "inner dimensions must match");
        let n = other.shape[1];

        let mut result = Tensor::zeros(&[m, n]);
        SimdOps::matmul_tiled(&self.data, &other.data, &mut result.data, m, k, n);
        result
    }

    /// Transpose a 2D tensor.
    pub fn transpose(&self) -> Tensor {
        assert_eq!(self.ndim(), 2, "transpose requires 2D tensor");
        let (m, n) = (self.shape[0], self.shape[1]);
        let mut data = vec![0.0; m * n];
        for i in 0..m {
            for j in 0..n {
                data[j * m + i] = self.data[i * n + j];
            }
        }
        Tensor {
            shape: vec![n, m],
            data,
        }
    }

    /// Apply a function element-wise.
    pub fn map<F: Fn(f64) -> f64>(&self, f: F) -> Tensor {
        let data: Vec<f64> = self.data.iter().map(|x| f(*x)).collect();
        Tensor {
            shape: self.shape.clone(),
            data,
        }
    }

    /// ReLU activation: max(0, x)
    pub fn relu(&self) -> Tensor {
        self.map(|x| x.max(0.0))
    }

    /// Sigmoid activation: 1 / (1 + e^(-x))
    pub fn sigmoid(&self) -> Tensor {
        self.map(|x| 1.0 / (1.0 + (-x).exp()))
    }

    /// Softmax over the entire tensor (flattened).
    pub fn softmax(&self) -> Tensor {
        let max_val = self.data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let exps: Vec<f64> = self.data.iter().map(|x| (x - max_val).exp()).collect();
        let sum: f64 = exps.iter().sum();
        let data: Vec<f64> = exps.iter().map(|x| x / sum).collect();
        Tensor {
            shape: self.shape.clone(),
            data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zeros() {
        let t = Tensor::zeros(&[2, 3]);
        assert_eq!(t.shape, vec![2, 3]);
        assert_eq!(t.numel(), 6);
        assert!(t.data.iter().all(|&x| x == 0.0));
    }

    #[test]
    fn test_ones() {
        let t = Tensor::ones(&[3]);
        assert_eq!(t.sum(), 3.0);
    }

    #[test]
    fn test_scalar() {
        let t = Tensor::scalar(42.0);
        assert_eq!(t.ndim(), 0);
        assert_eq!(t.numel(), 1);
        assert_eq!(t.get_flat(0), 42.0);
    }

    #[test]
    fn test_vector() {
        let t = Tensor::vector(vec![1.0, 2.0, 3.0]);
        assert_eq!(t.ndim(), 1);
        assert_eq!(t.shape[0], 3);
    }

    #[test]
    fn test_add() {
        let a = Tensor::vector(vec![1.0, 2.0, 3.0]);
        let b = Tensor::vector(vec![4.0, 5.0, 6.0]);
        let c = a.add(&b);
        assert_eq!(c.data, vec![5.0, 7.0, 9.0]);
    }

    #[test]
    fn test_mul() {
        let a = Tensor::vector(vec![1.0, 2.0, 3.0]);
        let b = Tensor::vector(vec![4.0, 5.0, 6.0]);
        let c = a.mul(&b);
        assert_eq!(c.data, vec![4.0, 10.0, 18.0]);
    }

    #[test]
    fn test_scale() {
        let a = Tensor::vector(vec![1.0, 2.0, 3.0]);
        let b = a.scale(2.0);
        assert_eq!(b.data, vec![2.0, 4.0, 6.0]);
    }

    #[test]
    fn test_dot() {
        let a = Tensor::vector(vec![1.0, 2.0, 3.0]);
        let b = Tensor::vector(vec![4.0, 5.0, 6.0]);
        assert_eq!(a.dot(&b), 32.0);
    }

    #[test]
    fn test_matmul() {
        // [1 2] × [5 6] = [1*5+2*7  1*6+2*8] = [19 22]
        // [3 4]   [7 8]   [3*5+4*7  3*6+4*8]   [43 50]
        let a = Tensor::from_data(&[2, 2], vec![1.0, 2.0, 3.0, 4.0]);
        let b = Tensor::from_data(&[2, 2], vec![5.0, 6.0, 7.0, 8.0]);
        let c = a.matmul(&b);
        assert_eq!(c.data, vec![19.0, 22.0, 43.0, 50.0]);
    }

    #[test]
    fn test_transpose() {
        let a = Tensor::from_data(&[2, 3], vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let t = a.transpose();
        assert_eq!(t.shape, vec![3, 2]);
        assert_eq!(t.data, vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0]);
    }

    #[test]
    fn test_relu() {
        let t = Tensor::vector(vec![-1.0, 0.0, 1.0, -0.5, 2.0]);
        let r = t.relu();
        assert_eq!(r.data, vec![0.0, 0.0, 1.0, 0.0, 2.0]);
    }

    #[test]
    fn test_sigmoid() {
        let t = Tensor::scalar(0.0);
        let s = t.sigmoid();
        assert!((s.get_flat(0) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_softmax() {
        let t = Tensor::vector(vec![1.0, 2.0, 3.0]);
        let s = t.softmax();
        assert!((s.sum() - 1.0).abs() < 1e-10);
        // Larger input → larger probability
        assert!(s.data[2] > s.data[1]);
        assert!(s.data[1] > s.data[0]);
    }

    #[test]
    fn test_mean() {
        let t = Tensor::vector(vec![2.0, 4.0, 6.0]);
        assert_eq!(t.mean(), 4.0);
    }
}
