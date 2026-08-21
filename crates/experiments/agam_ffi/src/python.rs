//! Python / NumPy Zero-Copy Buffer Protocol Interop Descriptor.

use serde::{Deserialize, Serialize};

/// NumPy / Python 3 Buffer Protocol descriptor for zero-copy tensor sharing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PyBufferDescriptor {
    pub data_ptr: usize,
    pub item_size: usize,
    pub format: String,
    pub shape: Vec<usize>,
    pub strides: Vec<isize>,
    pub readonly: bool,
}

impl PyBufferDescriptor {
    /// Create a standard C-contiguous 1D/2D/3D tensor buffer descriptor.
    pub fn new_c_contiguous(
        data_ptr: usize,
        item_size: usize,
        format: impl Into<String>,
        shape: Vec<usize>,
        readonly: bool,
    ) -> Self {
        let mut strides = Vec::with_capacity(shape.len());
        let mut current_stride = item_size as isize;

        for &dim in shape.iter().rev() {
            strides.push(current_stride);
            current_stride *= dim as isize;
        }
        strides.reverse();

        Self {
            data_ptr,
            item_size,
            format: format.into(),
            shape,
            strides,
            readonly,
        }
    }

    /// Total number of scalar elements in the buffer.
    pub fn total_elements(&self) -> usize {
        if self.shape.is_empty() {
            0
        } else {
            self.shape.iter().product()
        }
    }

    /// Total memory payload in bytes.
    pub fn total_bytes(&self) -> usize {
        self.total_elements() * self.item_size
    }

    /// Check if the buffer is C-contiguous.
    pub fn is_c_contiguous(&self) -> bool {
        if self.shape.is_empty() {
            return true;
        }
        let mut expected_stride = self.item_size as isize;
        for (&dim, &stride) in self.shape.iter().rev().zip(self.strides.iter().rev()) {
            if stride != expected_stride {
                return false;
            }
            expected_stride *= dim as isize;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_numpy_buffer_descriptor_strides_and_contiguity() {
        let buf = PyBufferDescriptor::new_c_contiguous(0x1000, 4, "f", vec![3, 4], false);
        assert_eq!(buf.total_elements(), 12);
        assert_eq!(buf.total_bytes(), 48);
        assert_eq!(buf.strides, vec![16, 4]);
        assert!(buf.is_c_contiguous());
    }
}
