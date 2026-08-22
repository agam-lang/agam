//! Zero-Copy Binary Serialization & Memory-Layout Engine.
//!
//! Provides ultra-low-latency zero-copy memory mapping, casting, alignment verification,
//! and endian-aware encoding/decoding for Agam structures and numeric tensors.

use std::mem::{align_of, size_of, size_of_val};

/// Errors encountered during zero-copy serialization/deserialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SerialError {
    BufferTooSmall {
        required: usize,
        provided: usize,
    },
    InvalidAlignment {
        required: usize,
        address: usize,
    },
    InvalidMagic {
        expected: [u8; 4],
        found: [u8; 4],
    },
    UnsupportedVersion {
        version: u32,
    },
    ChecksumMismatch {
        expected: u32,
        computed: u32,
    },
    PayloadSizeMismatch {
        header_size: u64,
        actual_size: usize,
    },
}

impl std::fmt::Display for SerialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BufferTooSmall { required, provided } => {
                write!(
                    f,
                    "Buffer too small: required {required} bytes, provided {provided} bytes"
                )
            }
            Self::InvalidAlignment { required, address } => {
                write!(
                    f,
                    "Invalid memory alignment: requires {required}-byte alignment, got address 0x{address:x}"
                )
            }
            Self::InvalidMagic { expected, found } => {
                write!(
                    f,
                    "Invalid format magic bytes: expected {expected:?}, found {found:?}"
                )
            }
            Self::UnsupportedVersion { version } => {
                write!(f, "Unsupported schema version: {version}")
            }
            Self::ChecksumMismatch { expected, computed } => {
                write!(
                    f,
                    "Checksum mismatch: expected 0x{expected:08x}, computed 0x{computed:08x}"
                )
            }
            Self::PayloadSizeMismatch {
                header_size,
                actual_size,
            } => {
                write!(
                    f,
                    "Payload size mismatch: header declares {header_size} bytes, got {actual_size} bytes"
                )
            }
        }
    }
}

impl std::error::Error for SerialError {}

/// Marker trait for Plain-Old-Data (POD) types safe for zero-copy memory reinterpretation.
///
/// # Safety
/// Implementing this trait guarantees that the type contains no uninitialized padding bytes,
/// has no pointer invariants, and any bit pattern of the specified size is a valid instance.
pub unsafe trait ZeroCopy: Sized + Copy {}

unsafe impl ZeroCopy for u8 {}
unsafe impl ZeroCopy for u16 {}
unsafe impl ZeroCopy for u32 {}
unsafe impl ZeroCopy for u64 {}
unsafe impl ZeroCopy for usize {}
unsafe impl ZeroCopy for i8 {}
unsafe impl ZeroCopy for i16 {}
unsafe impl ZeroCopy for i32 {}
unsafe impl ZeroCopy for i64 {}
unsafe impl ZeroCopy for isize {}
unsafe impl ZeroCopy for f32 {}
unsafe impl ZeroCopy for f64 {}
unsafe impl ZeroCopy for bool {}

unsafe impl<T: ZeroCopy, const N: usize> ZeroCopy for [T; N] {}

/// Cast a raw byte slice to a typed reference with strict size and alignment validation.
pub fn from_bytes<T: ZeroCopy>(bytes: &[u8]) -> Result<&T, SerialError> {
    let size = size_of::<T>();
    let align = align_of::<T>();

    if bytes.len() < size {
        return Err(SerialError::BufferTooSmall {
            required: size,
            provided: bytes.len(),
        });
    }

    let ptr = bytes.as_ptr();
    let addr = ptr as usize;
    if !addr.is_multiple_of(align) {
        return Err(SerialError::InvalidAlignment {
            required: align,
            address: addr,
        });
    }

    // SAFETY: Size and alignment checked, T implements ZeroCopy
    unsafe { Ok(&*(ptr as *const T)) }
}

/// Cast a raw byte slice to a typed contiguous slice with strict alignment validation.
pub fn from_bytes_slice<T: ZeroCopy>(bytes: &[u8]) -> Result<&[T], SerialError> {
    let size = size_of::<T>();
    let align = align_of::<T>();

    if size == 0 {
        return Ok(&[]);
    }

    if !bytes.len().is_multiple_of(size) {
        return Err(SerialError::BufferTooSmall {
            required: ((bytes.len() / size) + 1) * size,
            provided: bytes.len(),
        });
    }

    let ptr = bytes.as_ptr();
    let addr = ptr as usize;
    if !addr.is_multiple_of(align) {
        return Err(SerialError::InvalidAlignment {
            required: align,
            address: addr,
        });
    }

    let count = bytes.len() / size;
    // SAFETY: Size and alignment checked, T implements ZeroCopy
    unsafe { Ok(std::slice::from_raw_parts(ptr as *const T, count)) }
}

/// View a typed reference as an immutable raw byte slice.
pub fn to_bytes<T: ZeroCopy>(val: &T) -> &[u8] {
    let size = size_of::<T>();
    let ptr = val as *const T as *const u8;
    // SAFETY: T is ZeroCopy and pointer is valid for `size` bytes
    unsafe { std::slice::from_raw_parts(ptr, size) }
}

/// View a typed slice as an immutable raw byte slice.
pub fn to_bytes_slice<T: ZeroCopy>(slice: &[T]) -> &[u8] {
    let size = size_of_val(slice);
    let ptr = slice.as_ptr() as *const u8;
    // SAFETY: T is ZeroCopy and memory region is contiguous and valid
    unsafe { std::slice::from_raw_parts(ptr, size) }
}

/// File / Network header for zero-copy serialized envelopes.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SerialEnvelopeHeader {
    pub magic: [u8; 4],
    pub version: u32,
    pub payload_length: u64,
    pub crc32: u32,
    pub _reserved: u32,
}

unsafe impl ZeroCopy for SerialEnvelopeHeader {}

pub const AGAM_SERIAL_MAGIC: [u8; 4] = *b"AGAM";
pub const CURRENT_SERIAL_VERSION: u32 = 1;

/// Simple, fast CRC32-IEEE checksum implementation.
pub fn compute_crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in bytes {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Encode a typed slice payload into an envelope with header and checksum.
pub fn encode_envelope<T: ZeroCopy>(payload: &[T]) -> Vec<u8> {
    let payload_bytes = to_bytes_slice(payload);
    let crc = compute_crc32(payload_bytes);

    let header = SerialEnvelopeHeader {
        magic: AGAM_SERIAL_MAGIC,
        version: CURRENT_SERIAL_VERSION,
        payload_length: payload_bytes.len() as u64,
        crc32: crc,
        _reserved: 0,
    };

    let header_bytes = to_bytes(&header);
    let mut result = Vec::with_capacity(header_bytes.len() + payload_bytes.len());
    result.extend_from_slice(header_bytes);
    result.extend_from_slice(payload_bytes);
    result
}

/// Decode and verify an envelope, returning the payload typed slice without allocations.
pub fn decode_envelope<T: ZeroCopy>(buffer: &[u8]) -> Result<&[T], SerialError> {
    let header_size = size_of::<SerialEnvelopeHeader>();
    if buffer.len() < header_size {
        return Err(SerialError::BufferTooSmall {
            required: header_size,
            provided: buffer.len(),
        });
    }

    let header = from_bytes::<SerialEnvelopeHeader>(&buffer[..header_size])?;

    if header.magic != AGAM_SERIAL_MAGIC {
        return Err(SerialError::InvalidMagic {
            expected: AGAM_SERIAL_MAGIC,
            found: header.magic,
        });
    }

    if header.version != CURRENT_SERIAL_VERSION {
        return Err(SerialError::UnsupportedVersion {
            version: header.version,
        });
    }

    let payload_bytes = &buffer[header_size..];
    if (payload_bytes.len() as u64) != header.payload_length {
        return Err(SerialError::PayloadSizeMismatch {
            header_size: header.payload_length,
            actual_size: payload_bytes.len(),
        });
    }

    let computed_crc = compute_crc32(payload_bytes);
    if computed_crc != header.crc32 {
        return Err(SerialError::ChecksumMismatch {
            expected: header.crc32,
            computed: computed_crc,
        });
    }

    from_bytes_slice::<T>(payload_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Vec3 {
        x: f32,
        y: f32,
        z: f32,
    }

    unsafe impl ZeroCopy for Vec3 {}

    #[test]
    fn test_zero_copy_primitive_roundtrip() {
        let val: u64 = 0x1122_3344_5566_7788;
        let bytes = to_bytes(&val);
        assert_eq!(bytes.len(), 8);

        let decoded = from_bytes::<u64>(bytes).expect("Decode u64");
        assert_eq!(*decoded, val);
    }

    #[test]
    fn test_zero_copy_struct_slice_roundtrip() {
        let vertices = vec![
            Vec3 {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            Vec3 {
                x: 4.0,
                y: 5.0,
                z: 6.0,
            },
            Vec3 {
                x: 7.0,
                y: 8.0,
                z: 9.0,
            },
        ];

        let bytes = to_bytes_slice(&vertices);
        assert_eq!(bytes.len(), 3 * size_of::<Vec3>());

        let decoded_slice = from_bytes_slice::<Vec3>(bytes).expect("Decode Vec3 slice");
        assert_eq!(decoded_slice.len(), 3);
        assert_eq!(decoded_slice[0], vertices[0]);
        assert_eq!(decoded_slice[2], vertices[2]);
    }

    #[test]
    fn test_envelope_encoding_and_checksum_verification() {
        let data: Vec<i32> = (0..100).collect();
        let encoded = encode_envelope(&data);

        let decoded: &[i32] = decode_envelope(&encoded).expect("Decode envelope");
        assert_eq!(decoded.len(), 100);
        assert_eq!(decoded[42], 42);
        assert_eq!(decoded[99], 99);

        // Corrupt a byte in payload
        let mut corrupted = encoded.clone();
        let last_idx = corrupted.len() - 1;
        corrupted[last_idx] ^= 0xFF;

        let err = decode_envelope::<i32>(&corrupted).expect_err("Should fail on checksum");
        match err {
            SerialError::ChecksumMismatch { .. } => {}
            other => panic!("Expected ChecksumMismatch, got {:?}", other),
        }
    }
}
