//! Minimal host-side GPU buffer API for the first stdlib GPU surface.
//!
//! This is an in-memory fallback contract that mirrors the compiler builtins
//! (`gpu_malloc`, `gpu_free`, `gpu_memcpy_to_device`, `gpu_memcpy_to_host`)
//! so higher layers can exercise the API even without a CUDA runtime.

use std::fmt;

/// A host-side stand-in for device memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuBuffer {
    bytes: Vec<u8>,
}

impl GpuBuffer {
    /// Buffer length in bytes.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Return whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Borrow the underlying bytes for tests and host-side tooling.
    pub fn as_slice(&self) -> &[u8] {
        &self.bytes
    }
}

/// Structured GPU API error for size mismatches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuError {
    pub operation: &'static str,
    pub expected_bytes: usize,
    pub actual_bytes: usize,
}

impl fmt::Display for GpuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} expected {} bytes but received {} bytes",
            self.operation, self.expected_bytes, self.actual_bytes
        )
    }
}

impl std::error::Error for GpuError {}

/// Allocate one logical GPU buffer.
pub fn gpu_malloc(size_bytes: usize) -> GpuBuffer {
    GpuBuffer {
        bytes: vec![0; size_bytes],
    }
}

/// Free one logical GPU buffer.
pub fn gpu_free(_buffer: GpuBuffer) -> i32 {
    0
}

/// Copy bytes from host memory into the logical device buffer.
pub fn gpu_memcpy_to_device(dst: &mut GpuBuffer, src: impl AsRef<[u8]>) -> Result<i32, GpuError> {
    let src = src.as_ref();
    if src.len() != dst.bytes.len() {
        return Err(GpuError {
            operation: "gpu_memcpy_to_device",
            expected_bytes: dst.bytes.len(),
            actual_bytes: src.len(),
        });
    }
    dst.bytes.copy_from_slice(src);
    Ok(0)
}

/// Copy bytes from the logical device buffer back into host memory.
pub fn gpu_memcpy_to_host(src: &GpuBuffer, dst: &mut [u8]) -> Result<i32, GpuError> {
    if dst.len() != src.bytes.len() {
        return Err(GpuError {
            operation: "gpu_memcpy_to_host",
            expected_bytes: src.bytes.len(),
            actual_bytes: dst.len(),
        });
    }
    dst.copy_from_slice(&src.bytes);
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_buffer_round_trip() {
        let mut buffer = gpu_malloc(4);
        let host_src = [1_u8, 2, 3, 4];
        let mut host_dst = [0_u8; 4];

        gpu_memcpy_to_device(&mut buffer, host_src).expect("copy to device");
        gpu_memcpy_to_host(&buffer, &mut host_dst).expect("copy to host");

        assert_eq!(buffer.len(), 4);
        assert_eq!(buffer.as_slice(), &host_src);
        assert_eq!(host_dst, host_src);
        assert_eq!(gpu_free(buffer), 0);
    }

    #[test]
    fn gpu_memcpy_rejects_size_mismatch() {
        let mut buffer = gpu_malloc(4);
        let err = gpu_memcpy_to_device(&mut buffer, [1_u8, 2, 3]).unwrap_err();
        assert_eq!(err.operation, "gpu_memcpy_to_device");
        assert_eq!(err.expected_bytes, 4);
        assert_eq!(err.actual_bytes, 3);
    }
}
