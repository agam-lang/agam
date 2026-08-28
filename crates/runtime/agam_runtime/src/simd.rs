//! Hardware SIMD Vector Engine, Cacheline-Aligned Allocator, and Math Kernels.
//!
//! Provides dynamic CPU feature detection (AVX-512F, AVX2, FMA, NEON),
//! 64-byte cacheline-aligned buffers (`AlignedBuffer<T, 64>`), and hardware-vectorized
//! arithmetic kernels (`add`, `mul`, `fma`, `dot`) with zero runtime panics.

#![deny(clippy::unwrap_used)]

use std::alloc::Layout;
use std::fmt;
use std::ptr::NonNull;
use std::sync::OnceLock;

use crate::hwinfo::{SimdTier, hwinfo};

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

/// Detected hardware vector instruction capabilities.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SimdCapabilities {
    pub has_avx512f: bool,
    pub has_avx2: bool,
    pub has_fma: bool,
    pub has_neon: bool,
    pub vector_width_bytes: usize,
}

/// Dynamic CPU feature detector.
pub struct CpuFeatureDetector;

impl CpuFeatureDetector {
    /// Probe hardware CPUID and architecture vector flags dynamically.
    pub fn detect() -> SimdCapabilities {
        #[cfg(target_arch = "x86_64")]
        {
            let has_avx512f = std::is_x86_feature_detected!("avx512f");
            let has_avx2 = std::is_x86_feature_detected!("avx2");
            let has_fma = std::is_x86_feature_detected!("fma");
            let vector_width_bytes = if has_avx512f {
                64
            } else if has_avx2 {
                32
            } else {
                16
            };
            SimdCapabilities {
                has_avx512f,
                has_avx2,
                has_fma,
                has_neon: false,
                vector_width_bytes,
            }
        }
        #[cfg(target_arch = "aarch64")]
        {
            SimdCapabilities {
                has_avx512f: false,
                has_avx2: false,
                has_fma: false,
                has_neon: true,
                vector_width_bytes: 16,
            }
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            SimdCapabilities {
                has_avx512f: false,
                has_avx2: false,
                has_fma: false,
                has_neon: false,
                vector_width_bytes: 8,
            }
        }
    }

    /// Retrieve the cached CPU capability descriptor.
    pub fn current() -> SimdCapabilities {
        static CAPABILITIES: OnceLock<SimdCapabilities> = OnceLock::new();
        *CAPABILITIES.get_or_init(Self::detect)
    }
}

/// Structured SIMD diagnostic error formatted in the Agam Nyāya voice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimdError {
    pub cause: String,
    pub context: String,
    pub remedy: String,
}

impl SimdError {
    pub fn new(
        cause: impl Into<String>,
        context: impl Into<String>,
        remedy: impl Into<String>,
    ) -> Self {
        Self {
            cause: cause.into(),
            context: context.into(),
            remedy: remedy.into(),
        }
    }
}

impl fmt::Display for SimdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SIMD Vector Diagnostic: {}\n  Context: {}\n  Remedy:  {}",
            self.cause, self.context, self.remedy
        )
    }
}

impl std::error::Error for SimdError {}

/// A 64-byte cacheline-aligned continuous buffer for SIMD/Tensor memory.
#[derive(Debug)]
pub struct AlignedBuffer<T, const ALIGN: usize = 64> {
    ptr: *mut T,
    len: usize,
    capacity: usize,
}

unsafe impl<T: Send, const ALIGN: usize> Send for AlignedBuffer<T, ALIGN> {}
unsafe impl<T: Sync, const ALIGN: usize> Sync for AlignedBuffer<T, ALIGN> {}

impl<T, const ALIGN: usize> AlignedBuffer<T, ALIGN> {
    /// Allocate an uninitialized aligned buffer with specific capacity.
    pub fn with_capacity(capacity: usize) -> Result<Self, SimdError> {
        if capacity == 0 {
            return Ok(Self {
                ptr: NonNull::dangling().as_ptr(),
                len: 0,
                capacity: 0,
            });
        }

        let elem_size = std::mem::size_of::<T>();
        let size = capacity.checked_mul(elem_size).ok_or_else(|| {
            SimdError::new(
                "Buffer capacity overflow",
                format!(
                    "Capacity {} * element size {} exceeds usize::MAX",
                    capacity, elem_size
                ),
                "Reduce requested vector allocation capacity",
            )
        })?;

        let align = ALIGN.max(std::mem::align_of::<T>());
        let layout = Layout::from_size_align(size, align).map_err(|e| {
            SimdError::new(
                format!("Invalid alignment layout: {}", e),
                format!("Requested size {} with alignment {}", size, align),
                "Ensure alignment is a non-zero power of two",
            )
        })?;

        let raw = unsafe { std::alloc::alloc(layout) as *mut T };
        if raw.is_null() {
            return Err(SimdError::new(
                "Out of memory allocating aligned buffer",
                format!(
                    "Failed to allocate {} bytes with {}-byte alignment",
                    size, align
                ),
                "Ensure sufficient virtual/physical RAM is available",
            ));
        }

        Ok(Self {
            ptr: raw,
            len: 0,
            capacity,
        })
    }

    /// Allocate and fill an aligned buffer with cloned values from a slice.
    pub fn from_slice(data: &[T]) -> Result<Self, SimdError>
    where
        T: Clone,
    {
        let mut buf = Self::with_capacity(data.len())?;
        for item in data {
            buf.push(item.clone())?;
        }
        Ok(buf)
    }

    /// Number of initialized elements.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if buffer is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Allocated element capacity.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Pointer to the underlying memory.
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self.ptr
    }

    /// Mutable pointer to the underlying memory.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.ptr
    }

    /// Immutably borrow as a continuous slice.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        if self.len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
        }
    }

    /// Mutably borrow as a continuous slice.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        if self.len == 0 {
            &mut []
        } else {
            unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
        }
    }

    /// Push an element to the buffer.
    pub fn push(&mut self, item: T) -> Result<(), SimdError> {
        if self.len >= self.capacity {
            let new_cap = if self.capacity == 0 {
                8
            } else {
                self.capacity * 2
            };
            self.grow(new_cap)?;
        }
        unsafe {
            std::ptr::write(self.ptr.add(self.len), item);
        }
        self.len += 1;
        Ok(())
    }

    fn grow(&mut self, new_capacity: usize) -> Result<(), SimdError> {
        let elem_size = std::mem::size_of::<T>();
        let new_size = new_capacity
            .checked_mul(elem_size)
            .ok_or_else(|| SimdError::new("Capacity overflow during growth", "", ""))?;
        let align = ALIGN.max(std::mem::align_of::<T>());
        let new_layout = Layout::from_size_align(new_size, align)
            .map_err(|e| SimdError::new(format!("Invalid layout: {}", e), "", ""))?;

        let new_ptr = unsafe { std::alloc::alloc(new_layout) as *mut T };
        if new_ptr.is_null() {
            return Err(SimdError::new(
                "Out of memory growing aligned buffer",
                "",
                "",
            ));
        }

        if self.len > 0 {
            unsafe {
                std::ptr::copy_nonoverlapping(self.ptr, new_ptr, self.len);
            }
        }

        if self.capacity > 0 {
            let old_size = self.capacity * elem_size;
            if let Ok(old_layout) = Layout::from_size_align(old_size, align) {
                unsafe {
                    std::alloc::dealloc(self.ptr as *mut u8, old_layout);
                }
            }
        }

        self.ptr = new_ptr;
        self.capacity = new_capacity;
        Ok(())
    }
}

impl<T, const ALIGN: usize> Drop for AlignedBuffer<T, ALIGN> {
    fn drop(&mut self) {
        if self.capacity > 0 && !self.ptr.is_null() {
            for i in 0..self.len {
                unsafe {
                    std::ptr::drop_in_place(self.ptr.add(i));
                }
            }
            let elem_size = std::mem::size_of::<T>();
            let size = self.capacity * elem_size;
            let align = ALIGN.max(std::mem::align_of::<T>());
            if let Ok(layout) = Layout::from_size_align(size, align) {
                unsafe {
                    std::alloc::dealloc(self.ptr as *mut u8, layout);
                }
            }
            self.ptr = std::ptr::null_mut();
            self.len = 0;
            self.capacity = 0;
        }
    }
}

impl<T: Clone, const ALIGN: usize> Clone for AlignedBuffer<T, ALIGN> {
    fn clone(&self) -> Self {
        Self::from_slice(self.as_slice()).unwrap_or_else(|_| Self {
            ptr: NonNull::dangling().as_ptr(),
            len: 0,
            capacity: 0,
        })
    }
}

// ---------------------------------------------------------------------------
// Hardware Vectorized Math Kernels
// ---------------------------------------------------------------------------

/// Vector addition: `dst[i] = a[i] + b[i]`.
pub fn simd_add_f32(a: &[f32], b: &[f32], dst: &mut [f32]) -> Result<(), SimdError> {
    if a.len() != b.len() || a.len() != dst.len() {
        return Err(SimdError::new(
            "Slice length mismatch for simd_add_f32",
            format!(
                "a.len={}, b.len={}, dst.len={}",
                a.len(),
                b.len(),
                dst.len()
            ),
            "Ensure input and destination slices have identical lengths",
        ));
    }

    let n = a.len();
    let mut i = 0;

    #[cfg(target_arch = "x86_64")]
    {
        let caps = CpuFeatureDetector::current();
        if caps.has_avx2 {
            while i + 8 <= n {
                unsafe {
                    let va = _mm256_loadu_ps(a.as_ptr().add(i));
                    let vb = _mm256_loadu_ps(b.as_ptr().add(i));
                    let vr = _mm256_add_ps(va, vb);
                    _mm256_storeu_ps(dst.as_mut_ptr().add(i), vr);
                }
                i += 8;
            }
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        while i + 4 <= n {
            unsafe {
                let va = vld1q_f32(a.as_ptr().add(i));
                let vb = vld1q_f32(b.as_ptr().add(i));
                let vr = vaddq_f32(va, vb);
                vst1q_f32(dst.as_mut_ptr().add(i), vr);
            }
            i += 4;
        }
    }

    // Remainder scalar loop
    while i < n {
        dst[i] = a[i] + b[i];
        i += 1;
    }

    Ok(())
}

/// Vector multiplication: `dst[i] = a[i] * b[i]`.
pub fn simd_mul_f32(a: &[f32], b: &[f32], dst: &mut [f32]) -> Result<(), SimdError> {
    if a.len() != b.len() || a.len() != dst.len() {
        return Err(SimdError::new(
            "Slice length mismatch for simd_mul_f32",
            format!(
                "a.len={}, b.len={}, dst.len={}",
                a.len(),
                b.len(),
                dst.len()
            ),
            "Ensure input and destination slices have identical lengths",
        ));
    }

    let n = a.len();
    let mut i = 0;

    #[cfg(target_arch = "x86_64")]
    {
        let caps = CpuFeatureDetector::current();
        if caps.has_avx2 {
            while i + 8 <= n {
                unsafe {
                    let va = _mm256_loadu_ps(a.as_ptr().add(i));
                    let vb = _mm256_loadu_ps(b.as_ptr().add(i));
                    let vr = _mm256_mul_ps(va, vb);
                    _mm256_storeu_ps(dst.as_mut_ptr().add(i), vr);
                }
                i += 8;
            }
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        while i + 4 <= n {
            unsafe {
                let va = vld1q_f32(a.as_ptr().add(i));
                let vb = vld1q_f32(b.as_ptr().add(i));
                let vr = vmulq_f32(va, vb);
                vst1q_f32(dst.as_mut_ptr().add(i), vr);
            }
            i += 4;
        }
    }

    while i < n {
        dst[i] = a[i] * b[i];
        i += 1;
    }

    Ok(())
}

/// Vector fused multiply-add: `dst[i] = a[i] * b[i] + c[i]`.
pub fn simd_fma_f32(a: &[f32], b: &[f32], c: &[f32], dst: &mut [f32]) -> Result<(), SimdError> {
    if a.len() != b.len() || a.len() != c.len() || a.len() != dst.len() {
        return Err(SimdError::new(
            "Slice length mismatch for simd_fma_f32",
            format!(
                "a.len={}, b.len={}, c.len={}, dst.len={}",
                a.len(),
                b.len(),
                c.len(),
                dst.len()
            ),
            "Ensure input and destination slices have identical lengths",
        ));
    }

    let n = a.len();
    let mut i = 0;

    #[cfg(target_arch = "x86_64")]
    {
        let caps = CpuFeatureDetector::current();
        if caps.has_fma && caps.has_avx2 {
            while i + 8 <= n {
                unsafe {
                    let va = _mm256_loadu_ps(a.as_ptr().add(i));
                    let vb = _mm256_loadu_ps(b.as_ptr().add(i));
                    let vc = _mm256_loadu_ps(c.as_ptr().add(i));
                    let vr = _mm256_fmadd_ps(va, vb, vc);
                    _mm256_storeu_ps(dst.as_mut_ptr().add(i), vr);
                }
                i += 8;
            }
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        while i + 4 <= n {
            unsafe {
                let va = vld1q_f32(a.as_ptr().add(i));
                let vb = vld1q_f32(b.as_ptr().add(i));
                let vc = vld1q_f32(c.as_ptr().add(i));
                let vr = vfmaq_f32(vc, va, vb);
                vst1q_f32(dst.as_mut_ptr().add(i), vr);
            }
            i += 4;
        }
    }

    while i < n {
        dst[i] = a[i] * b[i] + c[i];
        i += 1;
    }

    Ok(())
}

/// Vector dot product: `Σ (a[i] * b[i])`.
pub fn simd_dot_f32(a: &[f32], b: &[f32]) -> Result<f32, SimdError> {
    if a.len() != b.len() {
        return Err(SimdError::new(
            "Slice length mismatch for simd_dot_f32",
            format!("a.len={}, b.len={}", a.len(), b.len()),
            "Ensure vector dot product slices have identical lengths",
        ));
    }

    let n = a.len();
    let mut sum = 0.0f32;
    let mut i = 0;

    #[cfg(target_arch = "x86_64")]
    {
        let caps = CpuFeatureDetector::current();
        if caps.has_fma && caps.has_avx2 {
            let mut acc = unsafe { _mm256_setzero_ps() };
            while i + 8 <= n {
                unsafe {
                    let va = _mm256_loadu_ps(a.as_ptr().add(i));
                    let vb = _mm256_loadu_ps(b.as_ptr().add(i));
                    acc = _mm256_fmadd_ps(va, vb, acc);
                }
                i += 8;
            }
            let mut tmp = [0.0f32; 8];
            unsafe {
                _mm256_storeu_ps(tmp.as_mut_ptr(), acc);
            }
            sum += tmp.iter().sum::<f32>();
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        let mut acc = unsafe { vdupq_n_f32(0.0) };
        while i + 4 <= n {
            unsafe {
                let va = vld1q_f32(a.as_ptr().add(i));
                let vb = vld1q_f32(b.as_ptr().add(i));
                acc = vfmaq_f32(acc, va, vb);
            }
            i += 4;
        }
        let mut tmp = [0.0f32; 4];
        unsafe {
            vst1q_f32(tmp.as_mut_ptr(), acc);
        }
        sum += tmp.iter().sum::<f32>();
    }

    while i < n {
        sum += a[i] * b[i];
        i += 1;
    }

    Ok(sum)
}

/// Vector addition: `dst[i] = a[i] + b[i]`.
pub fn simd_add_f64(a: &[f64], b: &[f64], dst: &mut [f64]) -> Result<(), SimdError> {
    if a.len() != b.len() || a.len() != dst.len() {
        return Err(SimdError::new(
            "Slice length mismatch for simd_add_f64",
            format!(
                "a.len={}, b.len={}, dst.len={}",
                a.len(),
                b.len(),
                dst.len()
            ),
            "Ensure input and destination slices have identical lengths",
        ));
    }

    let n = a.len();
    let mut i = 0;

    #[cfg(target_arch = "x86_64")]
    {
        let caps = CpuFeatureDetector::current();
        if caps.has_avx2 {
            while i + 4 <= n {
                unsafe {
                    let va = _mm256_loadu_pd(a.as_ptr().add(i));
                    let vb = _mm256_loadu_pd(b.as_ptr().add(i));
                    let vr = _mm256_add_pd(va, vb);
                    _mm256_storeu_pd(dst.as_mut_ptr().add(i), vr);
                }
                i += 4;
            }
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        while i + 2 <= n {
            unsafe {
                let va = vld1q_f64(a.as_ptr().add(i));
                let vb = vld1q_f64(b.as_ptr().add(i));
                let vr = vaddq_f64(va, vb);
                vst1q_f64(dst.as_mut_ptr().add(i), vr);
            }
            i += 2;
        }
    }

    while i < n {
        dst[i] = a[i] + b[i];
        i += 1;
    }

    Ok(())
}

/// Vector multiplication: `dst[i] = a[i] * b[i]`.
pub fn simd_mul_f64(a: &[f64], b: &[f64], dst: &mut [f64]) -> Result<(), SimdError> {
    if a.len() != b.len() || a.len() != dst.len() {
        return Err(SimdError::new(
            "Slice length mismatch for simd_mul_f64",
            format!(
                "a.len={}, b.len={}, dst.len={}",
                a.len(),
                b.len(),
                dst.len()
            ),
            "Ensure input and destination slices have identical lengths",
        ));
    }

    let n = a.len();
    let mut i = 0;

    #[cfg(target_arch = "x86_64")]
    {
        let caps = CpuFeatureDetector::current();
        if caps.has_avx2 {
            while i + 4 <= n {
                unsafe {
                    let va = _mm256_loadu_pd(a.as_ptr().add(i));
                    let vb = _mm256_loadu_pd(b.as_ptr().add(i));
                    let vr = _mm256_mul_pd(va, vb);
                    _mm256_storeu_pd(dst.as_mut_ptr().add(i), vr);
                }
                i += 4;
            }
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        while i + 2 <= n {
            unsafe {
                let va = vld1q_f64(a.as_ptr().add(i));
                let vb = vld1q_f64(b.as_ptr().add(i));
                let vr = vmulq_f64(va, vb);
                vst1q_f64(dst.as_mut_ptr().add(i), vr);
            }
            i += 2;
        }
    }

    while i < n {
        dst[i] = a[i] * b[i];
        i += 1;
    }

    Ok(())
}

/// Vector fused multiply-add: `dst[i] = a[i] * b[i] + c[i]`.
pub fn simd_fma_f64(a: &[f64], b: &[f64], c: &[f64], dst: &mut [f64]) -> Result<(), SimdError> {
    if a.len() != b.len() || a.len() != c.len() || a.len() != dst.len() {
        return Err(SimdError::new(
            "Slice length mismatch for simd_fma_f64",
            format!(
                "a.len={}, b.len={}, c.len={}, dst.len={}",
                a.len(),
                b.len(),
                c.len(),
                dst.len()
            ),
            "Ensure input and destination slices have identical lengths",
        ));
    }

    let n = a.len();
    let mut i = 0;

    #[cfg(target_arch = "x86_64")]
    {
        let caps = CpuFeatureDetector::current();
        if caps.has_fma && caps.has_avx2 {
            while i + 4 <= n {
                unsafe {
                    let va = _mm256_loadu_pd(a.as_ptr().add(i));
                    let vb = _mm256_loadu_pd(b.as_ptr().add(i));
                    let vc = _mm256_loadu_pd(c.as_ptr().add(i));
                    let vr = _mm256_fmadd_pd(va, vb, vc);
                    _mm256_storeu_pd(dst.as_mut_ptr().add(i), vr);
                }
                i += 4;
            }
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        while i + 2 <= n {
            unsafe {
                let va = vld1q_f64(a.as_ptr().add(i));
                let vb = vld1q_f64(b.as_ptr().add(i));
                let vc = vld1q_f64(c.as_ptr().add(i));
                let vr = vfmaq_f64(vc, va, vb);
                vst1q_f64(dst.as_mut_ptr().add(i), vr);
            }
            i += 2;
        }
    }

    while i < n {
        dst[i] = a[i] * b[i] + c[i];
        i += 1;
    }

    Ok(())
}

/// Vector dot product: `Σ (a[i] * b[i])`.
pub fn simd_dot_f64(a: &[f64], b: &[f64]) -> Result<f64, SimdError> {
    if a.len() != b.len() {
        return Err(SimdError::new(
            "Slice length mismatch for simd_dot_f64",
            format!("a.len={}, b.len={}", a.len(), b.len()),
            "Ensure vector dot product slices have identical lengths",
        ));
    }

    let n = a.len();
    let mut sum = 0.0f64;
    let mut i = 0;

    #[cfg(target_arch = "x86_64")]
    {
        let caps = CpuFeatureDetector::current();
        if caps.has_fma && caps.has_avx2 {
            let mut acc = unsafe { _mm256_setzero_pd() };
            while i + 4 <= n {
                unsafe {
                    let va = _mm256_loadu_pd(a.as_ptr().add(i));
                    let vb = _mm256_loadu_pd(b.as_ptr().add(i));
                    acc = _mm256_fmadd_pd(va, vb, acc);
                }
                i += 4;
            }
            let mut tmp = [0.0f64; 4];
            unsafe {
                _mm256_storeu_pd(tmp.as_mut_ptr(), acc);
            }
            sum += tmp.iter().sum::<f64>();
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        let mut acc = unsafe { vdupq_n_f64(0.0) };
        while i + 2 <= n {
            unsafe {
                let va = vld1q_f64(a.as_ptr().add(i));
                let vb = vld1q_f64(b.as_ptr().add(i));
                acc = vfmaq_f64(acc, va, vb);
            }
            i += 2;
        }
        let mut tmp = [0.0f64; 2];
        unsafe {
            vst1q_f64(tmp.as_mut_ptr(), acc);
        }
        sum += tmp.iter().sum::<f64>();
    }

    while i < n {
        sum += a[i] * b[i];
        i += 1;
    }

    Ok(sum)
}

// ---------------------------------------------------------------------------
// Legacy / Ergonomic SimdOps API
// ---------------------------------------------------------------------------

/// Portable SIMD-accelerated vector operations on contiguous f64 slices.
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
        let _ = simd_add_f64(&a[..n], &b[..n], &mut out[..n]);
    }

    /// Element-wise subtraction: out[i] = a[i] - b[i].
    #[inline]
    pub fn sub(a: &[f64], b: &[f64], out: &mut [f64]) {
        let n = a.len().min(b.len()).min(out.len());
        for i in 0..n {
            out[i] = a[i] - b[i];
        }
    }

    /// Element-wise multiply: out[i] = a[i] * b[i].
    #[inline]
    pub fn mul(a: &[f64], b: &[f64], out: &mut [f64]) {
        let n = a.len().min(b.len()).min(out.len());
        let _ = simd_mul_f64(&a[..n], &b[..n], &mut out[..n]);
    }

    /// Fused multiply-add: out[i] = a[i] * b[i] + c[i].
    #[inline]
    pub fn fma(a: &[f64], b: &[f64], c: &[f64], out: &mut [f64]) {
        let n = a.len().min(b.len()).min(c.len()).min(out.len());
        let _ = simd_fma_f64(&a[..n], &b[..n], &c[..n], &mut out[..n]);
    }

    /// Dot product: Σ a[i] * b[i].
    #[inline]
    pub fn dot(a: &[f64], b: &[f64]) -> f64 {
        let n = a.len().min(b.len());
        simd_dot_f64(&a[..n], &b[..n]).unwrap_or(0.0)
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

    /// Euclidean distance between two vectors: ‖a - b‖₂.
    #[inline]
    pub fn distance(a: &[f64], b: &[f64]) -> f64 {
        let n = a.len().min(b.len());
        let mut sum_sq = 0.0;
        for i in 0..n {
            let diff = a[i] - b[i];
            sum_sq += diff * diff;
        }
        sum_sq.sqrt()
    }

    /// Cache-tiled matrix multiplication: C = A × B where A is m×k, B is k×n, C is m×n.
    pub fn matmul_tiled(a: &[f64], b: &[f64], c: &mut [f64], m: usize, k: usize, n: usize) {
        const BLOCK: usize = 32;
        for el in c.iter_mut().take(m * n) {
            *el = 0.0;
        }

        for ii in (0..m).step_by(BLOCK) {
            let i_end = (ii + BLOCK).min(m);
            for kk in (0..k).step_by(BLOCK) {
                let k_end = (kk + BLOCK).min(k);
                for jj in (0..n).step_by(BLOCK) {
                    let j_end = (jj + BLOCK).min(n);
                    for i in ii..i_end {
                        for p in kk..k_end {
                            let a_val = a[i * k + p];
                            for j in jj..j_end {
                                c[i * n + j] += a_val * b[p * n + j];
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Alignment hint for vector allocations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignmentHint {
    None,
    SimdWidth,
    SimdVector,
    CacheLine,
    Page,
    Custom(usize),
}

impl AlignmentHint {
    pub const fn bytes(self) -> usize {
        match self {
            Self::None => 1,
            Self::SimdWidth | Self::SimdVector => 32,
            Self::CacheLine => 64,
            Self::Page => 4096,
            Self::Custom(b) => b,
        }
    }

    pub const fn align_up(self, offset: usize) -> usize {
        let align = self.bytes();
        (offset + align - 1) & !(align - 1)
    }
}

/// Runtime dispatch target selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchTarget {
    Auto,
    Scalar,
    Sse2,
    Avx2,
    Avx512,
    Neon,
}

impl DispatchTarget {
    pub fn resolve(self) -> SimdTier {
        match self {
            Self::Auto => SimdOps::tier(),
            Self::Scalar => SimdTier::Scalar,
            Self::Sse2 => SimdTier::Sse2,
            Self::Avx2 => SimdTier::Avx2,
            Self::Avx512 => SimdTier::Avx512,
            Self::Neon => SimdTier::Neon,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_cpu_feature_detection_is_deterministic() {
        let cap1 = CpuFeatureDetector::detect();
        let cap2 = CpuFeatureDetector::current();
        assert_eq!(cap1, cap2);
        assert!(cap1.vector_width_bytes >= 8);
    }

    #[test]
    fn test_aligned_buffer_64_byte_alignment() {
        let sizes = [1, 15, 64, 1024];
        for &size in &sizes {
            let buf = AlignedBuffer::<f32, 64>::with_capacity(size);
            assert!(buf.is_ok());
            if let Ok(b) = buf {
                assert_eq!((b.as_ptr() as usize) % 64, 0);
            }
        }
    }

    #[test]
    fn test_simd_vector_fma_f32_and_f64_accuracy() {
        let a_f32 = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        let b_f32 = [2.0f32, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let c_f32 = [0.5f32, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5];
        let mut dst_f32 = [0.0f32; 9];

        let res_f32 = simd_fma_f32(&a_f32, &b_f32, &c_f32, &mut dst_f32);
        assert!(res_f32.is_ok());
        for i in 0..9 {
            let expected = a_f32[i] * b_f32[i] + c_f32[i];
            assert!((dst_f32[i] - expected).abs() < 1e-6);
        }

        let dot_f32 = simd_dot_f32(&a_f32, &b_f32);
        assert!(dot_f32.is_ok());
        if let Ok(d) = dot_f32 {
            let expected: f32 = a_f32.iter().zip(b_f32.iter()).map(|(x, y)| x * y).sum();
            assert!((d - expected).abs() < 1e-6);
        }

        let a_f64 = [1.0f64, 2.0, 3.0, 4.0, 5.0];
        let b_f64 = [10.0f64, 20.0, 30.0, 40.0, 50.0];
        let c_f64 = [1.5f64, 2.5, 3.5, 4.5, 5.5];
        let mut dst_f64 = [0.0f64; 5];

        let res_f64 = simd_fma_f64(&a_f64, &b_f64, &c_f64, &mut dst_f64);
        assert!(res_f64.is_ok());
        for i in 0..5 {
            let expected = a_f64[i] * b_f64[i] + c_f64[i];
            assert!((dst_f64[i] - expected).abs() < 1e-10);
        }

        let dot_f64 = simd_dot_f64(&a_f64, &b_f64);
        assert!(dot_f64.is_ok());
        if let Ok(d) = dot_f64 {
            let expected: f64 = a_f64.iter().zip(b_f64.iter()).map(|(x, y)| x * y).sum();
            assert!((d - expected).abs() < 1e-10);
        }
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
    fn test_simd_arbitrary_length_and_remainder_loop_correctness() {
        // Test all sizes from 0 to 65 plus selected prime/odd sizes up to 257
        let test_sizes: Vec<usize> = (0..=65)
            .chain([71, 79, 83, 89, 97, 127, 128, 129, 255, 256, 257])
            .collect();

        for &n in &test_sizes {
            let mut a_f32 = Vec::with_capacity(n);
            let mut b_f32 = Vec::with_capacity(n);
            let mut c_f32 = Vec::with_capacity(n);
            let mut a_f64 = Vec::with_capacity(n);
            let mut b_f64 = Vec::with_capacity(n);
            let mut c_f64 = Vec::with_capacity(n);

            for i in 0..n {
                let v = ((i * 17 + 3) % 100) as f32 * 0.25;
                let u = ((i * 31 + 7) % 50) as f32 * 0.5;
                let w = ((i * 13 + 1) % 20) as f32 * 0.1;
                a_f32.push(v);
                b_f32.push(u);
                c_f32.push(w);
                a_f64.push(v as f64);
                b_f64.push(u as f64);
                c_f64.push(w as f64);
            }

            // Test f32 add
            let mut dst_add_f32 = vec![0.0f32; n];
            assert!(simd_add_f32(&a_f32, &b_f32, &mut dst_add_f32).is_ok());
            for i in 0..n {
                assert_eq!(dst_add_f32[i], a_f32[i] + b_f32[i]);
            }

            // Test f32 mul
            let mut dst_mul_f32 = vec![0.0f32; n];
            assert!(simd_mul_f32(&a_f32, &b_f32, &mut dst_mul_f32).is_ok());
            for i in 0..n {
                assert_eq!(dst_mul_f32[i], a_f32[i] * b_f32[i]);
            }

            // Test f32 fma
            let mut dst_fma_f32 = vec![0.0f32; n];
            assert!(simd_fma_f32(&a_f32, &b_f32, &c_f32, &mut dst_fma_f32).is_ok());
            for i in 0..n {
                let expected = a_f32[i] * b_f32[i] + c_f32[i];
                assert!((dst_fma_f32[i] - expected).abs() < 1e-5);
            }

            // Test f32 dot
            let dot_f32 = simd_dot_f32(&a_f32, &b_f32);
            assert!(dot_f32.is_ok());
            if let Ok(d) = dot_f32 {
                let expected: f32 = a_f32.iter().zip(b_f32.iter()).map(|(x, y)| x * y).sum();
                assert!((d - expected).abs() < 1e-4 * (n as f32 + 1.0));
            }

            // Test f64 add
            let mut dst_add_f64 = vec![0.0f64; n];
            assert!(simd_add_f64(&a_f64, &b_f64, &mut dst_add_f64).is_ok());
            for i in 0..n {
                assert_eq!(dst_add_f64[i], a_f64[i] + b_f64[i]);
            }

            // Test f64 mul
            let mut dst_mul_f64 = vec![0.0f64; n];
            assert!(simd_mul_f64(&a_f64, &b_f64, &mut dst_mul_f64).is_ok());
            for i in 0..n {
                assert_eq!(dst_mul_f64[i], a_f64[i] * b_f64[i]);
            }

            // Test f64 fma
            let mut dst_fma_f64 = vec![0.0f64; n];
            assert!(simd_fma_f64(&a_f64, &b_f64, &c_f64, &mut dst_fma_f64).is_ok());
            for i in 0..n {
                let expected = a_f64[i] * b_f64[i] + c_f64[i];
                assert!((dst_fma_f64[i] - expected).abs() < 1e-9);
            }

            // Test f64 dot
            let dot_f64 = simd_dot_f64(&a_f64, &b_f64);
            assert!(dot_f64.is_ok());
            if let Ok(d) = dot_f64 {
                let expected: f64 = a_f64.iter().zip(b_f64.iter()).map(|(x, y)| x * y).sum();
                assert!((d - expected).abs() < 1e-9 * (n as f64 + 1.0));
            }
        }
    }
}
