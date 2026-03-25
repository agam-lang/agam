//! NumPy-like N-dimensional array operations.
//!
//! Extends the `Tensor` with NumPy-style operations:
//! - Broadcasting, reshape, flatten, squeeze
//! - arange, linspace, meshgrid
//! - Reduction: argmax, argmin, cumsum
//! - Elementwise: abs, clip, where
//! - Random array generation
//!
//! All operations use contiguous f64 arrays for cache-friendly access.

use crate::tensor::Tensor;

/// Create a 1D tensor with evenly spaced values: [start, start+step, ..., stop).
pub fn arange(start: f64, stop: f64, step: f64) -> Tensor {
    let n = ((stop - start) / step).ceil() as usize;
    let data: Vec<f64> = (0..n).map(|i| start + i as f64 * step).collect();
    Tensor::vector(data)
}

/// Create a 1D tensor with n evenly spaced values in [start, stop].
pub fn linspace(start: f64, stop: f64, n: usize) -> Tensor {
    if n <= 1 { return Tensor::vector(vec![start]); }
    let step = (stop - start) / (n - 1) as f64;
    let data: Vec<f64> = (0..n).map(|i| start + i as f64 * step).collect();
    Tensor::vector(data)
}

/// Reshape a tensor (total elements must match).
pub fn reshape(t: &Tensor, new_shape: &[usize]) -> Tensor {
    let new_size: usize = new_shape.iter().product();
    assert_eq!(t.numel(), new_size, "reshape: total elements must match");
    Tensor::from_data(new_shape, t.data.clone())
}

/// Flatten a tensor to 1D.
pub fn flatten(t: &Tensor) -> Tensor {
    Tensor::vector(t.data.clone())
}

/// Squeeze: remove dimensions of size 1.
pub fn squeeze(t: &Tensor) -> Tensor {
    let new_shape: Vec<usize> = t.shape.iter().filter(|&&s| s > 1).cloned().collect();
    if new_shape.is_empty() {
        Tensor::scalar(t.data[0])
    } else {
        Tensor::from_data(&new_shape, t.data.clone())
    }
}

/// Argmax: index of the maximum element.
pub fn argmax(t: &Tensor) -> usize {
    t.data.iter().enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(i, _)| i)
        .unwrap()
}

/// Argmin: index of the minimum element.
pub fn argmin(t: &Tensor) -> usize {
    t.data.iter().enumerate()
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(i, _)| i)
        .unwrap()
}

/// Cumulative sum along the flattened tensor.
pub fn cumsum(t: &Tensor) -> Tensor {
    let mut data = Vec::with_capacity(t.numel());
    let mut acc = 0.0;
    for &v in &t.data {
        acc += v;
        data.push(acc);
    }
    Tensor { shape: t.shape.clone(), data }
}

/// Element-wise absolute value.
pub fn abs(t: &Tensor) -> Tensor {
    t.map(|x| x.abs())
}

/// Clip values to [lo, hi].
pub fn clip(t: &Tensor, lo: f64, hi: f64) -> Tensor {
    t.map(|x| x.max(lo).min(hi))
}

/// Element-wise where: result[i] = if cond[i] { a[i] } else { b[i] }
pub fn where_cond(cond: &[bool], a: &Tensor, b: &Tensor) -> Tensor {
    assert_eq!(cond.len(), a.numel());
    assert_eq!(a.numel(), b.numel());
    let data: Vec<f64> = cond.iter().zip(a.data.iter().zip(&b.data))
        .map(|(&c, (&av, &bv))| if c { av } else { bv })
        .collect();
    Tensor { shape: a.shape.clone(), data }
}

/// Stack: combine multiple 1D tensors into a 2D tensor (row-wise).
pub fn stack(tensors: &[Tensor]) -> Tensor {
    let cols = tensors[0].numel();
    for t in tensors {
        assert_eq!(t.numel(), cols, "all tensors must have same size");
    }
    let rows = tensors.len();
    let data: Vec<f64> = tensors.iter().flat_map(|t| t.data.iter().cloned()).collect();
    Tensor::from_data(&[rows, cols], data)
}

/// Concatenate: join multiple 1D tensors end-to-end.
pub fn concatenate(tensors: &[Tensor]) -> Tensor {
    let data: Vec<f64> = tensors.iter().flat_map(|t| t.data.iter().cloned()).collect();
    Tensor::vector(data)
}

/// Outer product: a ⊗ b → matrix where M[i,j] = a[i] * b[j]
pub fn outer(a: &Tensor, b: &Tensor) -> Tensor {
    assert_eq!(a.ndim(), 1);
    assert_eq!(b.ndim(), 1);
    let m = a.numel();
    let n = b.numel();
    let mut data = Vec::with_capacity(m * n);
    for i in 0..m {
        for j in 0..n {
            data.push(a.data[i] * b.data[j]);
        }
    }
    Tensor::from_data(&[m, n], data)
}

/// Norm (L2 / Euclidean).
pub fn norm(t: &Tensor) -> f64 {
    t.data.iter().map(|x| x * x).sum::<f64>().sqrt()
}

/// Normalize to unit norm.
pub fn normalize(t: &Tensor) -> Tensor {
    let n = norm(t);
    if n < 1e-15 { return t.clone(); }
    t.scale(1.0 / n)
}

/// Fill a tensor with a constant value.
pub fn full(shape: &[usize], value: f64) -> Tensor {
    let size: usize = shape.iter().product();
    Tensor::from_data(shape, vec![value; size])
}

/// Eye: identity matrix.
pub fn eye(n: usize) -> Tensor {
    let mut data = vec![0.0; n * n];
    for i in 0..n { data[i * n + i] = 1.0; }
    Tensor::from_data(&[n, n], data)
}

/// Diagonal: extract or create diagonal.
pub fn diag(values: &[f64]) -> Tensor {
    let n = values.len();
    let mut data = vec![0.0; n * n];
    for i in 0..n { data[i * n + i] = values[i]; }
    Tensor::from_data(&[n, n], data)
}

/// Variance of all elements.
pub fn variance(t: &Tensor) -> f64 {
    let mean = t.mean();
    t.data.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / t.numel() as f64
}

/// Standard deviation of all elements.
pub fn std_dev(t: &Tensor) -> f64 {
    variance(t).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arange() {
        let t = arange(0.0, 5.0, 1.0);
        assert_eq!(t.data, vec![0.0, 1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_linspace() {
        let t = linspace(0.0, 1.0, 5);
        assert_eq!(t.numel(), 5);
        assert!((t.data[0] - 0.0).abs() < 1e-10);
        assert!((t.data[4] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_reshape() {
        let t = Tensor::vector(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let r = reshape(&t, &[2, 3]);
        assert_eq!(r.shape, vec![2, 3]);
        assert_eq!(r.data, t.data);
    }

    #[test]
    fn test_flatten() {
        let t = Tensor::from_data(&[2, 3], vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let f = flatten(&t);
        assert_eq!(f.ndim(), 1);
        assert_eq!(f.numel(), 6);
    }

    #[test]
    fn test_argmax_argmin() {
        let t = Tensor::vector(vec![3.0, 1.0, 4.0, 1.0, 5.0]);
        assert_eq!(argmax(&t), 4);
        assert_eq!(argmin(&t), 1);
    }

    #[test]
    fn test_cumsum() {
        let t = Tensor::vector(vec![1.0, 2.0, 3.0, 4.0]);
        let c = cumsum(&t);
        assert_eq!(c.data, vec![1.0, 3.0, 6.0, 10.0]);
    }

    #[test]
    fn test_clip() {
        let t = Tensor::vector(vec![-2.0, 0.0, 3.0, 5.0]);
        let c = clip(&t, 0.0, 4.0);
        assert_eq!(c.data, vec![0.0, 0.0, 3.0, 4.0]);
    }

    #[test]
    fn test_where_cond() {
        let cond = vec![true, false, true, false];
        let a = Tensor::vector(vec![1.0, 2.0, 3.0, 4.0]);
        let b = Tensor::vector(vec![10.0, 20.0, 30.0, 40.0]);
        let r = where_cond(&cond, &a, &b);
        assert_eq!(r.data, vec![1.0, 20.0, 3.0, 40.0]);
    }

    #[test]
    fn test_stack() {
        let a = Tensor::vector(vec![1.0, 2.0]);
        let b = Tensor::vector(vec![3.0, 4.0]);
        let s = stack(&[a, b]);
        assert_eq!(s.shape, vec![2, 2]);
        assert_eq!(s.data, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_concatenate() {
        let a = Tensor::vector(vec![1.0, 2.0]);
        let b = Tensor::vector(vec![3.0, 4.0, 5.0]);
        let c = concatenate(&[a, b]);
        assert_eq!(c.data, vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn test_outer() {
        let a = Tensor::vector(vec![1.0, 2.0]);
        let b = Tensor::vector(vec![3.0, 4.0, 5.0]);
        let o = outer(&a, &b);
        assert_eq!(o.shape, vec![2, 3]);
        assert_eq!(o.data, vec![3.0, 4.0, 5.0, 6.0, 8.0, 10.0]);
    }

    #[test]
    fn test_norm_normalize() {
        let t = Tensor::vector(vec![3.0, 4.0]);
        assert!((norm(&t) - 5.0).abs() < 1e-10);
        let n = normalize(&t);
        assert!((norm(&n) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_eye() {
        let e = eye(3);
        assert_eq!(e.shape, vec![3, 3]);
        assert_eq!(e.data[0], 1.0);
        assert_eq!(e.data[1], 0.0);
        assert_eq!(e.data[4], 1.0);
    }

    #[test]
    fn test_diag() {
        let d = diag(&[1.0, 2.0, 3.0]);
        assert_eq!(d.shape, vec![3, 3]);
        assert_eq!(d.data[0], 1.0);
        assert_eq!(d.data[4], 2.0);
        assert_eq!(d.data[8], 3.0);
    }

    #[test]
    fn test_variance_std() {
        let t = Tensor::vector(vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]);
        assert_eq!(variance(&t), 4.0);
        assert_eq!(std_dev(&t), 2.0);
    }
}
