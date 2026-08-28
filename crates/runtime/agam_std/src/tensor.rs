//! N-dimensional Tensor type and zero-copy Strided Views for Agam.
//!
//! Provides a shape-aware, contiguous-memory tensor with support for
//! element-wise operations, broadcasting, zero-copy views, and basic linear algebra.
//!
//! This is the foundation for Agam's native AI/ML capabilities.

use agam_runtime::simd::SimdOps;
use std::fmt;

/// Errors arising from invalid tensor operations or shape mismatches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TensorError {
    ShapeMismatch {
        expected: Vec<usize>,
        actual: Vec<usize>,
    },
    DimensionMismatch {
        expected: usize,
        actual: usize,
    },
    DataLengthMismatch {
        expected: usize,
        actual: usize,
    },
    IncompatibleInnerDimensions {
        left: usize,
        right: usize,
    },
    OutOfBoundsIndex {
        index: usize,
        total: usize,
    },
}

impl fmt::Display for TensorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TensorError::ShapeMismatch { expected, actual } => {
                write!(
                    f,
                    "tensor shape mismatch: expected {:?}, got {:?}",
                    expected, actual
                )
            }
            TensorError::DimensionMismatch { expected, actual } => {
                write!(
                    f,
                    "tensor dimension mismatch: expected {}D, got {}D",
                    expected, actual
                )
            }
            TensorError::DataLengthMismatch { expected, actual } => {
                write!(
                    f,
                    "tensor data length mismatch: expected {} elements, got {}",
                    expected, actual
                )
            }
            TensorError::IncompatibleInnerDimensions { left, right } => {
                write!(
                    f,
                    "incompatible inner dimensions for matrix multiplication: {} vs {}",
                    left, right
                )
            }
            TensorError::OutOfBoundsIndex { index, total } => {
                write!(f, "tensor index out of bounds: {} >= {}", index, total)
            }
        }
    }
}

impl std::error::Error for TensorError {}

/// Compute canonical row-major strides for a given tensor shape.
pub fn default_strides(shape: &[usize]) -> Vec<usize> {
    if shape.is_empty() {
        return Vec::new();
    }
    let mut strides = vec![1; shape.len()];
    for i in (0..shape.len().saturating_sub(1)).rev() {
        strides[i] = strides[i + 1] * shape[i + 1];
    }
    strides
}

/// Zero-copy strided view over an underlying tensor buffer.
#[derive(Clone, Copy, Debug)]
pub struct TensorView<'a> {
    pub data: &'a [f64],
    pub shape: &'a [usize],
    pub strides: &'a [usize],
    pub offset: usize,
}

impl<'a> TensorView<'a> {
    pub fn new(data: &'a [f64], shape: &'a [usize], strides: &'a [usize], offset: usize) -> Self {
        Self {
            data,
            shape,
            strides,
            offset,
        }
    }

    /// Read an element by multi-dimensional coordinates in O(1) time without copies.
    pub fn get(&self, indices: &[usize]) -> Option<f64> {
        if indices.len() != self.shape.len() {
            return None;
        }
        let mut flat_idx = self.offset;
        for (i, &idx) in indices.iter().enumerate() {
            if idx >= self.shape[i] {
                return None;
            }
            flat_idx += idx * self.strides[i];
        }
        self.data.get(flat_idx).copied()
    }
}

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

    /// Fallible constructor from flat data and shape.
    pub fn try_from_data(shape: &[usize], data: Vec<f64>) -> Result<Self, TensorError> {
        let expected: usize = shape.iter().product();
        if data.len() != expected {
            return Err(TensorError::DataLengthMismatch {
                expected,
                actual: data.len(),
            });
        }
        Ok(Self {
            shape: shape.to_vec(),
            data,
        })
    }

    /// Create a tensor from flat data and shape (panicking on invalid input).
    pub fn from_data(shape: &[usize], data: Vec<f64>) -> Self {
        Self::try_from_data(shape, data).expect("data length must match shape product")
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

    /// Borrow as a zero-copy strided view.
    pub fn as_view<'a>(&'a self, strides: &'a [usize]) -> TensorView<'a> {
        TensorView::new(&self.data, &self.shape, strides, 0)
    }

    /// Element-wise addition (fallible).
    pub fn try_add(&self, other: &Tensor) -> Result<Tensor, TensorError> {
        if self.shape != other.shape {
            return Err(TensorError::ShapeMismatch {
                expected: self.shape.clone(),
                actual: other.shape.clone(),
            });
        }
        let mut data = vec![0.0; self.numel()];
        SimdOps::add(&self.data, &other.data, &mut data);
        Ok(Tensor {
            shape: self.shape.clone(),
            data,
        })
    }

    /// Element-wise addition. Shapes must match.
    pub fn add(&self, other: &Tensor) -> Tensor {
        self.try_add(other).expect("shapes must match for add")
    }

    /// Element-wise subtraction (fallible).
    pub fn try_sub(&self, other: &Tensor) -> Result<Tensor, TensorError> {
        if self.shape != other.shape {
            return Err(TensorError::ShapeMismatch {
                expected: self.shape.clone(),
                actual: other.shape.clone(),
            });
        }
        let mut data = vec![0.0; self.numel()];
        SimdOps::sub(&self.data, &other.data, &mut data);
        Ok(Tensor {
            shape: self.shape.clone(),
            data,
        })
    }

    /// Element-wise subtraction. Shapes must match.
    pub fn sub(&self, other: &Tensor) -> Tensor {
        self.try_sub(other).expect("shapes must match for sub")
    }

    /// Element-wise multiplication (fallible).
    pub fn try_mul(&self, other: &Tensor) -> Result<Tensor, TensorError> {
        if self.shape != other.shape {
            return Err(TensorError::ShapeMismatch {
                expected: self.shape.clone(),
                actual: other.shape.clone(),
            });
        }
        let mut data = vec![0.0; self.numel()];
        SimdOps::mul(&self.data, &other.data, &mut data);
        Ok(Tensor {
            shape: self.shape.clone(),
            data,
        })
    }

    /// Element-wise multiplication (Hadamard). Shapes must match.
    pub fn mul(&self, other: &Tensor) -> Tensor {
        self.try_mul(other).expect("shapes must match for mul")
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
        if self.data.is_empty() {
            0.0
        } else {
            self.sum() / self.numel() as f64
        }
    }

    /// Dot product (for 1D tensors / vectors, fallible).
    pub fn try_dot(&self, other: &Tensor) -> Result<f64, TensorError> {
        if self.ndim() != 1 {
            return Err(TensorError::DimensionMismatch {
                expected: 1,
                actual: self.ndim(),
            });
        }
        if other.ndim() != 1 {
            return Err(TensorError::DimensionMismatch {
                expected: 1,
                actual: other.ndim(),
            });
        }
        if self.shape[0] != other.shape[0] {
            return Err(TensorError::DataLengthMismatch {
                expected: self.shape[0],
                actual: other.shape[0],
            });
        }
        Ok(SimdOps::dot(&self.data, &other.data))
    }

    /// Dot product (for 1D tensors / vectors).
    pub fn dot(&self, other: &Tensor) -> f64 {
        self.try_dot(other).unwrap_or(0.0)
    }

    /// Fused multiply-add: `out = self * other + bias` (fallible).
    pub fn try_fma(&self, other: &Tensor, bias: &Tensor) -> Result<Tensor, TensorError> {
        if self.shape != other.shape || self.shape != bias.shape {
            return Err(TensorError::ShapeMismatch {
                expected: self.shape.clone(),
                actual: other.shape.clone(),
            });
        }
        let mut data = vec![0.0; self.numel()];
        SimdOps::fma(&self.data, &other.data, &bias.data, &mut data);
        Ok(Tensor {
            shape: self.shape.clone(),
            data,
        })
    }

    /// Fused multiply-add: `out = self * other + bias`.
    pub fn fma(&self, other: &Tensor, bias: &Tensor) -> Tensor {
        self.try_fma(other, bias)
            .unwrap_or_else(|_| Tensor::zeros(&self.shape))
    }

    /// Export tensor contiguous data to a 64-byte hardware cacheline-aligned buffer.
    pub fn to_aligned_buffer(
        &self,
    ) -> Result<agam_runtime::simd::AlignedBuffer<f64, 64>, TensorError> {
        agam_runtime::simd::AlignedBuffer::from_slice(&self.data).map_err(|_| {
            TensorError::DataLengthMismatch {
                expected: self.numel(),
                actual: 0,
            }
        })
    }

    /// Construct a tensor from a 64-byte hardware aligned buffer.
    pub fn from_aligned_buffer(
        shape: &[usize],
        buf: &agam_runtime::simd::AlignedBuffer<f64, 64>,
    ) -> Result<Self, TensorError> {
        let expected: usize = shape.iter().product();
        if buf.len() != expected {
            return Err(TensorError::DataLengthMismatch {
                expected,
                actual: buf.len(),
            });
        }
        Ok(Self {
            shape: shape.to_vec(),
            data: buf.as_slice().to_vec(),
        })
    }

    /// Matrix multiplication for 2D tensors (fallible).
    /// self: [M, K], other: [K, N] → result: [M, N]
    pub fn try_matmul(&self, other: &Tensor) -> Result<Tensor, TensorError> {
        if self.ndim() != 2 {
            return Err(TensorError::DimensionMismatch {
                expected: 2,
                actual: self.ndim(),
            });
        }
        if other.ndim() != 2 {
            return Err(TensorError::DimensionMismatch {
                expected: 2,
                actual: other.ndim(),
            });
        }
        let m = self.shape[0];
        let k = self.shape[1];
        if k != other.shape[0] {
            return Err(TensorError::IncompatibleInnerDimensions {
                left: k,
                right: other.shape[0],
            });
        }
        let n = other.shape[1];

        let mut result = Tensor::zeros(&[m, n]);
        SimdOps::matmul_tiled(&self.data, &other.data, &mut result.data, m, k, n);
        Ok(result)
    }

    /// Matrix multiplication for 2D tensors.
    pub fn matmul(&self, other: &Tensor) -> Tensor {
        self.try_matmul(other)
            .expect("valid matrix multiplication shapes")
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
    fn test_strided_view_access() {
        let t = Tensor::from_data(&[2, 3], vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0]);
        let strides = default_strides(&t.shape);
        let view = t.as_view(&strides);

        assert_eq!(view.get(&[0, 0]), Some(10.0));
        assert_eq!(view.get(&[0, 2]), Some(30.0));
        assert_eq!(view.get(&[1, 1]), Some(50.0));
        assert_eq!(view.get(&[2, 0]), None); // Out of bounds
    }

    #[test]
    fn test_try_from_data_error_handling() {
        let res = Tensor::try_from_data(&[2, 2], vec![1.0, 2.0, 3.0]);
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            TensorError::DataLengthMismatch {
                expected: 4,
                actual: 3
            }
        );
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
        assert!(s.data[2] > s.data[1]);
        assert!(s.data[1] > s.data[0]);
    }

    #[test]
    fn test_mean() {
        let t = Tensor::vector(vec![2.0, 4.0, 6.0]);
        assert_eq!(t.mean(), 4.0);
    }

    #[test]
    fn test_tensor_operations_leverage_aligned_simd() {
        let n = 10_000;
        let mut a_data = Vec::with_capacity(n);
        let mut b_data = Vec::with_capacity(n);
        let mut c_data = Vec::with_capacity(n);
        for i in 0..n {
            a_data.push((i % 100) as f64 * 0.1);
            b_data.push(((i + 1) % 50) as f64 * 0.2);
            c_data.push(1.0);
        }

        let a = Tensor::vector(a_data.clone());
        let b = Tensor::vector(b_data.clone());
        let c = Tensor::vector(c_data.clone());

        // Test add
        let add_res = a.add(&b);
        assert_eq!(add_res.numel(), n);
        for i in 0..n {
            assert!((add_res.get_flat(i) - (a_data[i] + b_data[i])).abs() < 1e-10);
        }

        // Test dot product
        let dot_res = a.dot(&b);
        let expected_dot: f64 = a_data.iter().zip(b_data.iter()).map(|(x, y)| x * y).sum();
        assert!((dot_res - expected_dot).abs() < 1e-8);

        // Test fma
        let fma_res = a.fma(&b, &c);
        assert_eq!(fma_res.numel(), n);
        for i in 0..n {
            assert!((fma_res.get_flat(i) - (a_data[i] * b_data[i] + c_data[i])).abs() < 1e-10);
        }

        // Test AlignedBuffer conversion
        let aligned_buf = a.to_aligned_buffer();
        assert!(aligned_buf.is_ok());
        if let Ok(buf) = aligned_buf {
            assert_eq!((buf.as_ptr() as usize) % 64, 0);
            assert_eq!(buf.len(), n);
            let reconstructed = Tensor::from_aligned_buffer(&[n], &buf);
            assert!(reconstructed.is_ok());
            if let Ok(t) = reconstructed {
                assert_eq!(t.data, a.data);
            }
        }
    }
}
