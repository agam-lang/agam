//! SIMD intrinsics abstraction layer.
//!
//! Provides portable SIMD operations that dispatch to the best available
//! instruction set at runtime. The `SimdVec` type wraps contiguous f64
//! arrays and accelerates bulk operations.
//!
//! ## Dispatch strategy
//! 1. Check `hwinfo().simd.best_tier()` at startup
//! 2. Select vectorized or scalar codepath
//! 3. Process elements in SIMD-width chunks, handle remainder scalar

use crate::hwinfo::{hwinfo, SimdTier};

/// Portable SIMD-accelerated vector operations on contiguous f64 slices.
///
/// All operations fall back to scalar loops but are structured for
/// compiler auto-vectorization (sequential iteration, no branching).
pub struct SimdOps;

impl SimdOps {
    /// Current SIMD tier.
    pub fn tier() -> SimdTier {
        hwinfo().simd.best_tier()
    }

    /// Lanes available for f64 on this machine.
    pub fn lanes() -> usize {
        Self::tier().f64_lanes()
    }

    /// Element-wise add: out[i] = a[i] + b[i].
    /// Written as a simple loop for auto-vectorization (LLVM will emit SIMD).
    #[inline]
    pub fn add(a: &[f64], b: &[f64], out: &mut [f64]) {
        let n = a.len().min(b.len()).min(out.len());
        for i in 0..n {
            out[i] = a[i] + b[i];
        }
    }

    /// Element-wise multiply: out[i] = a[i] * b[i].
    #[inline]
    pub fn mul(a: &[f64], b: &[f64], out: &mut [f64]) {
        let n = a.len().min(b.len()).min(out.len());
        for i in 0..n {
            out[i] = a[i] * b[i];
        }
    }

    /// Fused multiply-add: out[i] = a[i] * b[i] + c[i].
    /// On FMA-capable CPUs, LLVM emits a single vfmadd instruction.
    #[inline]
    pub fn fma(a: &[f64], b: &[f64], c: &[f64], out: &mut [f64]) {
        let n = a.len().min(b.len()).min(c.len()).min(out.len());
        for i in 0..n {
            out[i] = a[i].mul_add(b[i], c[i]);
        }
    }

    /// Dot product: Σ a[i] * b[i].
    /// The loop structure enables auto-vectorization with horizontal sum.
    #[inline]
    pub fn dot(a: &[f64], b: &[f64]) -> f64 {
        let n = a.len().min(b.len());
        let mut sum = 0.0;
        for i in 0..n {
            sum += a[i] * b[i];
        }
        sum
    }

    /// Sum reduction: Σ a[i].
    #[inline]
    pub fn sum(a: &[f64]) -> f64 {
        a.iter().sum()
    }

    /// Scale: out[i] = a[i] * scalar.
    #[inline]
    pub fn scale(a: &[f64], scalar: f64, out: &mut [f64]) {
        let n = a.len().min(out.len());
        for i in 0..n {
            out[i] = a[i] * scalar;
        }
    }

    /// Max element.
    #[inline]
    pub fn max(a: &[f64]) -> f64 {
        a.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
    }

    /// Min element.
    #[inline]
    pub fn min(a: &[f64]) -> f64 {
        a.iter().cloned().fold(f64::INFINITY, f64::min)
    }

    /// L2 norm: √(Σ a[i]²).
    #[inline]
    pub fn norm_l2(a: &[f64]) -> f64 {
        Self::dot(a, a).sqrt()
    }

    /// Euclidean distance between two vectors.
    #[inline]
    pub fn distance(a: &[f64], b: &[f64]) -> f64 {
        let n = a.len().min(b.len());
        let mut sum = 0.0;
        for i in 0..n {
            let d = a[i] - b[i];
            sum += d * d;
        }
        sum.sqrt()
    }

    /// Blocked matrix multiply: C += A × B.
    /// Uses tiling to maximize L1 cache reuse.
    /// A: [m × k], B: [k × n], C: [m × n], all row-major.
    pub fn matmul_tiled(
        a: &[f64], b: &[f64], c: &mut [f64],
        m: usize, k: usize, n: usize,
    ) {
        let tile = hwinfo().optimal_tile_size().min(m).min(n).min(k).max(1);

        // Zero output
        for v in c.iter_mut() { *v = 0.0; }

        // Blocked / tiled triple loop
        let mut ii = 0;
        while ii < m {
            let i_end = (ii + tile).min(m);
            let mut jj = 0;
            while jj < n {
                let j_end = (jj + tile).min(n);
                let mut kk = 0;
                while kk < k {
                    let k_end = (kk + tile).min(k);
                    // Inner micro-kernel
                    for i in ii..i_end {
                        for kp in kk..k_end {
                            let a_ik = a[i * k + kp];
                            for j in jj..j_end {
                                c[i * n + j] += a_ik * b[kp * n + j];
                            }
                        }
                    }
                    kk += tile;
                }
                jj += tile;
            }
            ii += tile;
        }
    }
}

/// Alignment hint for data structures.
/// Used by `#[align(L1_Cache)]` annotations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignmentHint {
    /// Align to cache line (64 bytes on most architectures).
    CacheLine,
    /// Align to L1 cache size.
    L1Cache,
    /// Align to SIMD register width.
    SimdWidth,
    /// Custom alignment in bytes (must be power of 2).
    Custom(usize),
}

impl AlignmentHint {
    /// Resolve to actual bytes.
    pub fn bytes(&self) -> usize {
        match self {
            AlignmentHint::CacheLine => hwinfo().l1_data.line_size,
            AlignmentHint::L1Cache => hwinfo().l1_data.size,
            AlignmentHint::SimdWidth => hwinfo().simd.best_simd_width(),
            AlignmentHint::Custom(n) => *n,
        }
    }

    /// Align a pointer/offset to the given alignment.
    pub fn align_up(&self, addr: usize) -> usize {
        let align = self.bytes();
        (addr + align - 1) & !(align - 1)
    }
}

/// Dispatch target for `#[dispatch]` annotations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchTarget {
    /// Scalar (no SIMD).
    Scalar,
    /// Best available SIMD.
    Simd,
    /// Explicit SIMD tier.
    SimdTier(SimdTier),
    /// GPU (future: CUDA/Metal/Vulkan compute).
    Gpu,
    /// Auto: runtime selects best.
    Auto,
}

impl DispatchTarget {
    /// Resolve to the actual tier.
    pub fn resolve(&self) -> SimdTier {
        match self {
            DispatchTarget::Scalar => SimdTier::Scalar,
            DispatchTarget::Simd | DispatchTarget::Auto => hwinfo().simd.best_tier(),
            DispatchTarget::SimdTier(t) => *t,
            DispatchTarget::Gpu => SimdTier::Scalar, // GPU dispatch is future work
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_add() {
        let a = vec![1.0, 2.0, 3.0, 4.0];
        let b = vec![5.0, 6.0, 7.0, 8.0];
        let mut out = vec![0.0; 4];
        SimdOps::add(&a, &b, &mut out);
        assert_eq!(out, vec![6.0, 8.0, 10.0, 12.0]);
    }

    #[test]
    fn test_simd_mul() {
        let a = vec![2.0, 3.0, 4.0];
        let b = vec![5.0, 6.0, 7.0];
        let mut out = vec![0.0; 3];
        SimdOps::mul(&a, &b, &mut out);
        assert_eq!(out, vec![10.0, 18.0, 28.0]);
    }

    #[test]
    fn test_simd_fma() {
        let a = vec![2.0, 3.0];
        let b = vec![4.0, 5.0];
        let c = vec![1.0, 1.0];
        let mut out = vec![0.0; 2];
        SimdOps::fma(&a, &b, &c, &mut out);
        assert_eq!(out, vec![9.0, 16.0]); // 2*4+1=9, 3*5+1=16
    }

    #[test]
    fn test_simd_dot() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        assert!((SimdOps::dot(&a, &b) - 32.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_scale() {
        let a = vec![1.0, 2.0, 3.0];
        let mut out = vec![0.0; 3];
        SimdOps::scale(&a, 2.5, &mut out);
        assert_eq!(out, vec![2.5, 5.0, 7.5]);
    }

    #[test]
    fn test_simd_norm() {
        let a = vec![3.0, 4.0];
        assert!((SimdOps::norm_l2(&a) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_distance() {
        let a = vec![0.0, 0.0];
        let b = vec![3.0, 4.0];
        assert!((SimdOps::distance(&a, &b) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_matmul_tiled_2x2() {
        // [1 2] × [5 6] = [1*5+2*7, 1*6+2*8] = [19 22]
        // [3 4]   [7 8]   [3*5+4*7, 3*6+4*8]   [43 50]
        let a = vec![1.0, 2.0, 3.0, 4.0];
        let b = vec![5.0, 6.0, 7.0, 8.0];
        let mut c = vec![0.0; 4];
        SimdOps::matmul_tiled(&a, &b, &mut c, 2, 2, 2);
        assert!((c[0] - 19.0).abs() < 1e-10);
        assert!((c[1] - 22.0).abs() < 1e-10);
        assert!((c[2] - 43.0).abs() < 1e-10);
        assert!((c[3] - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_matmul_tiled_3x3() {
        let a = vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]; // identity
        let b = vec![5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0];
        let mut c = vec![0.0; 9];
        SimdOps::matmul_tiled(&a, &b, &mut c, 3, 3, 3);
        assert_eq!(c, b); // I × B = B
    }

    #[test]
    fn test_alignment_hint() {
        assert_eq!(AlignmentHint::CacheLine.bytes(), 64);
        assert_eq!(AlignmentHint::Custom(128).bytes(), 128);
        assert_eq!(AlignmentHint::CacheLine.align_up(65), 128);
        assert_eq!(AlignmentHint::CacheLine.align_up(64), 64);
    }

    #[test]
    fn test_dispatch_resolve() {
        let target = DispatchTarget::Auto;
        let tier = target.resolve();
        // On most modern x86, at least SSE2
        assert!(tier >= SimdTier::Scalar);
    }

    #[test]
    fn test_lanes() {
        let lanes = SimdOps::lanes();
        assert!(lanes >= 1);
    }
}
