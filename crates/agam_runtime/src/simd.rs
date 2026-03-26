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

#![allow(unsafe_op_in_unsafe_fn)]

use crate::hwinfo::{hwinfo, SimdTier};

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

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
    #[inline]
    pub fn add(a: &[f64], b: &[f64], out: &mut [f64]) {
        let n = a.len().min(b.len()).min(out.len());
        dispatch_binary(a, b, &mut out[..n], BinaryOp::Add);
    }

    /// Element-wise subtraction: out[i] = a[i] - b[i].
    #[inline]
    pub fn sub(a: &[f64], b: &[f64], out: &mut [f64]) {
        let n = a.len().min(b.len()).min(out.len());
        dispatch_binary(a, b, &mut out[..n], BinaryOp::Sub);
    }

    /// Element-wise multiply: out[i] = a[i] * b[i].
    #[inline]
    pub fn mul(a: &[f64], b: &[f64], out: &mut [f64]) {
        let n = a.len().min(b.len()).min(out.len());
        dispatch_binary(a, b, &mut out[..n], BinaryOp::Mul);
    }

    /// Fused multiply-add: out[i] = a[i] * b[i] + c[i].
    #[inline]
    pub fn fma(a: &[f64], b: &[f64], c: &[f64], out: &mut [f64]) {
        let n = a.len().min(b.len()).min(c.len()).min(out.len());
        dispatch_fma(a, b, c, &mut out[..n]);
    }

    /// Dot product: Σ a[i] * b[i].
    #[inline]
    pub fn dot(a: &[f64], b: &[f64]) -> f64 {
        let n = a.len().min(b.len());
        dispatch_dot(&a[..n], &b[..n])
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
        dispatch_scale(&a[..n], scalar, &mut out[..n]);
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
                            let row_start = i * n + jj;
                            let row_end = i * n + j_end;
                            let b_start = kp * n + jj;
                            let b_end = kp * n + j_end;
                            axpy_inplace(
                                &mut c[row_start..row_end],
                                &b[b_start..b_end],
                                a_ik,
                            );
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

fn scalar_add(a: &[f64], b: &[f64], out: &mut [f64]) {
    for i in 0..out.len() {
        out[i] = a[i] + b[i];
    }
}

fn scalar_sub(a: &[f64], b: &[f64], out: &mut [f64]) {
    for i in 0..out.len() {
        out[i] = a[i] - b[i];
    }
}

fn scalar_mul(a: &[f64], b: &[f64], out: &mut [f64]) {
    for i in 0..out.len() {
        out[i] = a[i] * b[i];
    }
}

fn scalar_fma(a: &[f64], b: &[f64], c: &[f64], out: &mut [f64]) {
    for i in 0..out.len() {
        out[i] = a[i].mul_add(b[i], c[i]);
    }
}

fn scalar_dot(a: &[f64], b: &[f64]) -> f64 {
    let mut sum = 0.0;
    for i in 0..a.len() {
        sum += a[i] * b[i];
    }
    sum
}

fn scalar_scale(a: &[f64], scalar: f64, out: &mut [f64]) {
    for i in 0..out.len() {
        out[i] = a[i] * scalar;
    }
}

fn scalar_axpy(out: &mut [f64], src: &[f64], scalar: f64) {
    for i in 0..out.len() {
        out[i] += src[i] * scalar;
    }
}

#[derive(Clone, Copy)]
enum BinaryOp {
    Add,
    Sub,
    Mul,
}

fn dispatch_binary(
    a: &[f64],
    b: &[f64],
    out: &mut [f64],
    op: BinaryOp,
) {
    let tier = SimdOps::tier();

    #[cfg(target_arch = "x86_64")]
    unsafe {
        match tier {
            SimdTier::Avx512 => {
                dispatch_x86_binary_avx512(a, b, out, op);
                return;
            }
            SimdTier::Avx | SimdTier::Avx2 => {
                dispatch_x86_binary_avx(a, b, out, op);
                return;
            }
            SimdTier::Sse2 | SimdTier::Sse42 => {
                dispatch_x86_binary_sse2(a, b, out, op);
                return;
            }
            _ => {}
        }
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        if matches!(tier, SimdTier::Neon) {
            dispatch_neon_binary(a, b, out, op);
            return;
        }
    }

    match op {
        BinaryOp::Add => scalar_add(a, b, out),
        BinaryOp::Sub => scalar_sub(a, b, out),
        BinaryOp::Mul => scalar_mul(a, b, out),
    }
}

fn dispatch_fma(a: &[f64], b: &[f64], c: &[f64], out: &mut [f64]) {
    let tier = SimdOps::tier();

    #[cfg(target_arch = "x86_64")]
    unsafe {
        if hwinfo().simd.fma && matches!(tier, SimdTier::Avx | SimdTier::Avx2 | SimdTier::Avx512) {
            fma_x86_fma(a, b, c, out);
            return;
        }
    }

    scalar_fma(a, b, c, out);
}

fn dispatch_dot(a: &[f64], b: &[f64]) -> f64 {
    let tier = SimdOps::tier();

    #[cfg(target_arch = "x86_64")]
    unsafe {
        match tier {
            SimdTier::Avx512 => return dot_x86_avx512(a, b),
            SimdTier::Avx | SimdTier::Avx2 => return dot_x86_avx(a, b),
            SimdTier::Sse2 | SimdTier::Sse42 => return dot_x86_sse2(a, b),
            _ => {}
        }
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        if matches!(tier, SimdTier::Neon) {
            return dot_neon(a, b);
        }
    }

    scalar_dot(a, b)
}

fn dispatch_scale(a: &[f64], scalar: f64, out: &mut [f64]) {
    let tier = SimdOps::tier();

    #[cfg(target_arch = "x86_64")]
    unsafe {
        match tier {
            SimdTier::Avx512 => {
                scale_x86_avx512(a, scalar, out);
                return;
            }
            SimdTier::Avx | SimdTier::Avx2 => {
                scale_x86_avx(a, scalar, out);
                return;
            }
            SimdTier::Sse2 | SimdTier::Sse42 => {
                scale_x86_sse2(a, scalar, out);
                return;
            }
            _ => {}
        }
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        if matches!(tier, SimdTier::Neon) {
            scale_neon(a, scalar, out);
            return;
        }
    }

    scalar_scale(a, scalar, out);
}

fn axpy_inplace(out: &mut [f64], src: &[f64], scalar: f64) {
    let tier = SimdOps::tier();

    #[cfg(target_arch = "x86_64")]
    unsafe {
        match tier {
            SimdTier::Avx512 => {
                axpy_x86_avx512(out, src, scalar);
                return;
            }
            SimdTier::Avx | SimdTier::Avx2 => {
                axpy_x86_avx(out, src, scalar);
                return;
            }
            SimdTier::Sse2 | SimdTier::Sse42 => {
                axpy_x86_sse2(out, src, scalar);
                return;
            }
            _ => {}
        }
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        if matches!(tier, SimdTier::Neon) {
            axpy_neon(out, src, scalar);
            return;
        }
    }

    scalar_axpy(out, src, scalar);
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
unsafe fn dispatch_x86_binary_avx512(a: &[f64], b: &[f64], out: &mut [f64], op: BinaryOp) {
    let limit = out.len() / 8 * 8;
    let mut i = 0;
    while i < limit {
        let va = _mm512_loadu_pd(a.as_ptr().add(i));
        let vb = _mm512_loadu_pd(b.as_ptr().add(i));
        let vr = match op {
            BinaryOp::Add => _mm512_add_pd(va, vb),
            BinaryOp::Sub => _mm512_sub_pd(va, vb),
            BinaryOp::Mul => _mm512_mul_pd(va, vb),
        };
        _mm512_storeu_pd(out.as_mut_ptr().add(i), vr);
        i += 8;
    }
    match op {
        BinaryOp::Add => scalar_add(&a[limit..], &b[limit..], &mut out[limit..]),
        BinaryOp::Sub => scalar_sub(&a[limit..], &b[limit..], &mut out[limit..]),
        BinaryOp::Mul => scalar_mul(&a[limit..], &b[limit..], &mut out[limit..]),
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx")]
unsafe fn dispatch_x86_binary_avx(a: &[f64], b: &[f64], out: &mut [f64], op: BinaryOp) {
    let limit = out.len() / 4 * 4;
    let mut i = 0;
    while i < limit {
        let va = _mm256_loadu_pd(a.as_ptr().add(i));
        let vb = _mm256_loadu_pd(b.as_ptr().add(i));
        let vr = match op {
            BinaryOp::Add => _mm256_add_pd(va, vb),
            BinaryOp::Sub => _mm256_sub_pd(va, vb),
            BinaryOp::Mul => _mm256_mul_pd(va, vb),
        };
        _mm256_storeu_pd(out.as_mut_ptr().add(i), vr);
        i += 4;
    }
    match op {
        BinaryOp::Add => scalar_add(&a[limit..], &b[limit..], &mut out[limit..]),
        BinaryOp::Sub => scalar_sub(&a[limit..], &b[limit..], &mut out[limit..]),
        BinaryOp::Mul => scalar_mul(&a[limit..], &b[limit..], &mut out[limit..]),
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
unsafe fn dispatch_x86_binary_sse2(a: &[f64], b: &[f64], out: &mut [f64], op: BinaryOp) {
    let limit = out.len() / 2 * 2;
    let mut i = 0;
    while i < limit {
        let va = _mm_loadu_pd(a.as_ptr().add(i));
        let vb = _mm_loadu_pd(b.as_ptr().add(i));
        let vr = match op {
            BinaryOp::Add => _mm_add_pd(va, vb),
            BinaryOp::Sub => _mm_sub_pd(va, vb),
            BinaryOp::Mul => _mm_mul_pd(va, vb),
        };
        _mm_storeu_pd(out.as_mut_ptr().add(i), vr);
        i += 2;
    }
    match op {
        BinaryOp::Add => scalar_add(&a[limit..], &b[limit..], &mut out[limit..]),
        BinaryOp::Sub => scalar_sub(&a[limit..], &b[limit..], &mut out[limit..]),
        BinaryOp::Mul => scalar_mul(&a[limit..], &b[limit..], &mut out[limit..]),
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx,fma")]
unsafe fn fma_x86_fma(a: &[f64], b: &[f64], c: &[f64], out: &mut [f64]) {
    let limit = out.len() / 4 * 4;
    let mut i = 0;
    while i < limit {
        let va = _mm256_loadu_pd(a.as_ptr().add(i));
        let vb = _mm256_loadu_pd(b.as_ptr().add(i));
        let vc = _mm256_loadu_pd(c.as_ptr().add(i));
        let vr = _mm256_fmadd_pd(va, vb, vc);
        _mm256_storeu_pd(out.as_mut_ptr().add(i), vr);
        i += 4;
    }
    scalar_fma(&a[limit..], &b[limit..], &c[limit..], &mut out[limit..]);
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
unsafe fn dot_x86_avx512(a: &[f64], b: &[f64]) -> f64 {
    let limit = a.len() / 8 * 8;
    let mut acc = _mm512_setzero_pd();
    let mut i = 0;
    while i < limit {
        let va = _mm512_loadu_pd(a.as_ptr().add(i));
        let vb = _mm512_loadu_pd(b.as_ptr().add(i));
        acc = _mm512_add_pd(acc, _mm512_mul_pd(va, vb));
        i += 8;
    }
    let mut lanes = [0.0; 8];
    _mm512_storeu_pd(lanes.as_mut_ptr(), acc);
    lanes.into_iter().sum::<f64>() + scalar_dot(&a[limit..], &b[limit..])
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx")]
unsafe fn dot_x86_avx(a: &[f64], b: &[f64]) -> f64 {
    let limit = a.len() / 4 * 4;
    let mut acc = _mm256_setzero_pd();
    let mut i = 0;
    while i < limit {
        let va = _mm256_loadu_pd(a.as_ptr().add(i));
        let vb = _mm256_loadu_pd(b.as_ptr().add(i));
        acc = _mm256_add_pd(acc, _mm256_mul_pd(va, vb));
        i += 4;
    }
    let mut lanes = [0.0; 4];
    _mm256_storeu_pd(lanes.as_mut_ptr(), acc);
    lanes.into_iter().sum::<f64>() + scalar_dot(&a[limit..], &b[limit..])
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
unsafe fn dot_x86_sse2(a: &[f64], b: &[f64]) -> f64 {
    let limit = a.len() / 2 * 2;
    let mut acc = _mm_setzero_pd();
    let mut i = 0;
    while i < limit {
        let va = _mm_loadu_pd(a.as_ptr().add(i));
        let vb = _mm_loadu_pd(b.as_ptr().add(i));
        acc = _mm_add_pd(acc, _mm_mul_pd(va, vb));
        i += 2;
    }
    let mut lanes = [0.0; 2];
    _mm_storeu_pd(lanes.as_mut_ptr(), acc);
    lanes.into_iter().sum::<f64>() + scalar_dot(&a[limit..], &b[limit..])
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
unsafe fn scale_x86_avx512(a: &[f64], scalar: f64, out: &mut [f64]) {
    let factor = _mm512_set1_pd(scalar);
    let limit = out.len() / 8 * 8;
    let mut i = 0;
    while i < limit {
        let va = _mm512_loadu_pd(a.as_ptr().add(i));
        _mm512_storeu_pd(out.as_mut_ptr().add(i), _mm512_mul_pd(va, factor));
        i += 8;
    }
    scalar_scale(&a[limit..], scalar, &mut out[limit..]);
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx")]
unsafe fn scale_x86_avx(a: &[f64], scalar: f64, out: &mut [f64]) {
    let factor = _mm256_set1_pd(scalar);
    let limit = out.len() / 4 * 4;
    let mut i = 0;
    while i < limit {
        let va = _mm256_loadu_pd(a.as_ptr().add(i));
        _mm256_storeu_pd(out.as_mut_ptr().add(i), _mm256_mul_pd(va, factor));
        i += 4;
    }
    scalar_scale(&a[limit..], scalar, &mut out[limit..]);
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
unsafe fn scale_x86_sse2(a: &[f64], scalar: f64, out: &mut [f64]) {
    let factor = _mm_set1_pd(scalar);
    let limit = out.len() / 2 * 2;
    let mut i = 0;
    while i < limit {
        let va = _mm_loadu_pd(a.as_ptr().add(i));
        _mm_storeu_pd(out.as_mut_ptr().add(i), _mm_mul_pd(va, factor));
        i += 2;
    }
    scalar_scale(&a[limit..], scalar, &mut out[limit..]);
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
unsafe fn axpy_x86_avx512(out: &mut [f64], src: &[f64], scalar: f64) {
    let factor = _mm512_set1_pd(scalar);
    let limit = out.len() / 8 * 8;
    let mut i = 0;
    while i < limit {
        let vd = _mm512_loadu_pd(out.as_ptr().add(i));
        let vs = _mm512_loadu_pd(src.as_ptr().add(i));
        let vr = _mm512_add_pd(vd, _mm512_mul_pd(vs, factor));
        _mm512_storeu_pd(out.as_mut_ptr().add(i), vr);
        i += 8;
    }
    scalar_axpy(&mut out[limit..], &src[limit..], scalar);
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx")]
unsafe fn axpy_x86_avx(out: &mut [f64], src: &[f64], scalar: f64) {
    let factor = _mm256_set1_pd(scalar);
    let limit = out.len() / 4 * 4;
    let mut i = 0;
    while i < limit {
        let vd = _mm256_loadu_pd(out.as_ptr().add(i));
        let vs = _mm256_loadu_pd(src.as_ptr().add(i));
        let vr = _mm256_add_pd(vd, _mm256_mul_pd(vs, factor));
        _mm256_storeu_pd(out.as_mut_ptr().add(i), vr);
        i += 4;
    }
    scalar_axpy(&mut out[limit..], &src[limit..], scalar);
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
unsafe fn axpy_x86_sse2(out: &mut [f64], src: &[f64], scalar: f64) {
    let factor = _mm_set1_pd(scalar);
    let limit = out.len() / 2 * 2;
    let mut i = 0;
    while i < limit {
        let vd = _mm_loadu_pd(out.as_ptr().add(i));
        let vs = _mm_loadu_pd(src.as_ptr().add(i));
        let vr = _mm_add_pd(vd, _mm_mul_pd(vs, factor));
        _mm_storeu_pd(out.as_mut_ptr().add(i), vr);
        i += 2;
    }
    scalar_axpy(&mut out[limit..], &src[limit..], scalar);
}

#[cfg(target_arch = "aarch64")]
unsafe fn dispatch_neon_binary(a: &[f64], b: &[f64], out: &mut [f64], op: BinaryOp) {
    let limit = out.len() / 2 * 2;
    let mut i = 0;
    while i < limit {
        let va = vld1q_f64(a.as_ptr().add(i));
        let vb = vld1q_f64(b.as_ptr().add(i));
        let vr = match op {
            BinaryOp::Add => vaddq_f64(va, vb),
            BinaryOp::Sub => vsubq_f64(va, vb),
            BinaryOp::Mul => vmulq_f64(va, vb),
        };
        vst1q_f64(out.as_mut_ptr().add(i), vr);
        i += 2;
    }
    match op {
        BinaryOp::Add => scalar_add(&a[limit..], &b[limit..], &mut out[limit..]),
        BinaryOp::Sub => scalar_sub(&a[limit..], &b[limit..], &mut out[limit..]),
        BinaryOp::Mul => scalar_mul(&a[limit..], &b[limit..], &mut out[limit..]),
    }
}

#[cfg(target_arch = "aarch64")]
unsafe fn dot_neon(a: &[f64], b: &[f64]) -> f64 {
    let limit = a.len() / 2 * 2;
    let mut acc = vdupq_n_f64(0.0);
    let mut i = 0;
    while i < limit {
        let va = vld1q_f64(a.as_ptr().add(i));
        let vb = vld1q_f64(b.as_ptr().add(i));
        acc = vaddq_f64(acc, vmulq_f64(va, vb));
        i += 2;
    }
    let mut lanes = [0.0; 2];
    vst1q_f64(lanes.as_mut_ptr(), acc);
    lanes.into_iter().sum::<f64>() + scalar_dot(&a[limit..], &b[limit..])
}

#[cfg(target_arch = "aarch64")]
unsafe fn scale_neon(a: &[f64], scalar: f64, out: &mut [f64]) {
    let factor = vdupq_n_f64(scalar);
    let limit = out.len() / 2 * 2;
    let mut i = 0;
    while i < limit {
        let va = vld1q_f64(a.as_ptr().add(i));
        vst1q_f64(out.as_mut_ptr().add(i), vmulq_f64(va, factor));
        i += 2;
    }
    scalar_scale(&a[limit..], scalar, &mut out[limit..]);
}

#[cfg(target_arch = "aarch64")]
unsafe fn axpy_neon(out: &mut [f64], src: &[f64], scalar: f64) {
    let factor = vdupq_n_f64(scalar);
    let limit = out.len() / 2 * 2;
    let mut i = 0;
    while i < limit {
        let vd = vld1q_f64(out.as_ptr().add(i));
        let vs = vld1q_f64(src.as_ptr().add(i));
        let vr = vaddq_f64(vd, vmulq_f64(vs, factor));
        vst1q_f64(out.as_mut_ptr().add(i), vr);
        i += 2;
    }
    scalar_axpy(&mut out[limit..], &src[limit..], scalar);
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
    fn test_simd_sub() {
        let a = vec![5.0, 7.0, 9.0];
        let b = vec![1.0, 2.0, 3.0];
        let mut out = vec![0.0; 3];
        SimdOps::sub(&a, &b, &mut out);
        assert_eq!(out, vec![4.0, 5.0, 6.0]);
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
