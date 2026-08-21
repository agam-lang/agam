//! Python / NumPy / PyTorch Zero-Copy Buffer Protocol & DLPack Interop.
//!
//! Implements memory descriptors for zero-copy tensor sharing between Agam,
//! NumPy, PyTorch (C10/ATen), and DLPack v0.8+ universal exchange protocols.

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

/// Supported element scalar data types in PyTorch / C10 tensors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PyTorchDtype {
    Float32,
    Float64,
    Float16,
    BFloat16,
    Int32,
    Int64,
    Int16,
    Int8,
    UInt8,
    Bool,
}

impl PyTorchDtype {
    /// Size of an individual element in bytes.
    pub fn item_size(&self) -> usize {
        match self {
            PyTorchDtype::Float64 | PyTorchDtype::Int64 => 8,
            PyTorchDtype::Float32 | PyTorchDtype::Int32 => 4,
            PyTorchDtype::Float16 | PyTorchDtype::BFloat16 | PyTorchDtype::Int16 => 2,
            PyTorchDtype::Int8 | PyTorchDtype::UInt8 | PyTorchDtype::Bool => 1,
        }
    }

    /// Return Python format code for struct packing / buffer protocol.
    pub fn format_code(&self) -> &'static str {
        match self {
            PyTorchDtype::Float64 => "d",
            PyTorchDtype::Float32 => "f",
            PyTorchDtype::Float16 | PyTorchDtype::BFloat16 => "e",
            PyTorchDtype::Int64 => "q",
            PyTorchDtype::Int32 => "i",
            PyTorchDtype::Int16 => "h",
            PyTorchDtype::Int8 => "b",
            PyTorchDtype::UInt8 => "B",
            PyTorchDtype::Bool => "?",
        }
    }
}

/// Target execution device for PyTorch tensors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PyTorchDevice {
    Cpu,
    Cuda(u32),
}

/// Zero-copy PyTorch (C10 / ATen) tensor descriptor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PyTorchTensorDescriptor {
    pub data_ptr: usize,
    pub dtype: PyTorchDtype,
    pub device: PyTorchDevice,
    pub shape: Vec<usize>,
    pub strides: Vec<isize>,
    pub requires_grad: bool,
}

impl PyTorchTensorDescriptor {
    /// Create a new C-contiguous PyTorch tensor descriptor.
    pub fn new_contiguous(
        data_ptr: usize,
        dtype: PyTorchDtype,
        device: PyTorchDevice,
        shape: Vec<usize>,
        requires_grad: bool,
    ) -> Self {
        let mut strides = Vec::with_capacity(shape.len());
        let mut current_stride = 1isize; // PyTorch strides are measured in elements

        for &dim in shape.iter().rev() {
            strides.push(current_stride);
            current_stride *= dim as isize;
        }
        strides.reverse();

        Self {
            data_ptr,
            dtype,
            device,
            shape,
            strides,
            requires_grad,
        }
    }

    /// Total number of elements.
    pub fn numel(&self) -> usize {
        if self.shape.is_empty() {
            0
        } else {
            self.shape.iter().product()
        }
    }

    /// Total size in bytes.
    pub fn total_bytes(&self) -> usize {
        self.numel() * self.dtype.item_size()
    }

    /// Check if tensor memory layout is C-contiguous.
    pub fn is_contiguous(&self) -> bool {
        if self.shape.is_empty() {
            return true;
        }
        let mut expected = 1isize;
        for (&dim, &stride) in self.shape.iter().rev().zip(self.strides.iter().rev()) {
            if stride != expected {
                return false;
            }
            expected *= dim as isize;
        }
        true
    }

    /// Convert to NumPy / Python standard buffer descriptor.
    pub fn to_buffer_descriptor(&self) -> PyBufferDescriptor {
        let item_size = self.dtype.item_size();
        let byte_strides = self
            .strides
            .iter()
            .map(|&s| s * (item_size as isize))
            .collect();
        PyBufferDescriptor {
            data_ptr: self.data_ptr,
            item_size,
            format: self.dtype.format_code().to_string(),
            shape: self.shape.clone(),
            strides: byte_strides,
            readonly: false,
        }
    }
}

// ── DLPack v0.8+ Standard C-ABI Exchange Layouts ──────────────────────────

/// DLPack device type enum (kDLCPU = 1, kDLCUDA = 2).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DLDeviceType {
    DLCPU = 1,
    DLCUDA = 2,
    DLCUDAHost = 3,
    DLOpenCL = 4,
    DLVulkan = 7,
    DLMetal = 8,
    DLVPI = 9,
    DLROCm = 10,
}

/// DLPack device descriptor.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DLDevice {
    pub device_type: DLDeviceType,
    pub device_id: i32,
}

/// DLPack data type code.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DLDataTypeCode {
    DLInt = 0,
    DLUInt = 1,
    DLFloat = 2,
    DLOpaqueHandle = 3,
    DLBfloat = 4,
    DLComplex = 5,
    DLBool = 6,
}

/// DLPack data type descriptor.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DLDataType {
    pub code: u8,
    pub bits: u8,
    pub lanes: u16,
}

/// DLPack tensor header struct.
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct DLTensor {
    pub data: *mut std::ffi::c_void,
    pub device: DLDevice,
    pub ndim: i32,
    pub dtype: DLDataType,
    pub shape: *mut i64,
    pub strides: *mut i64,
    pub byte_offset: u64,
}

/// DLManagedTensor represents a tensor that is managed by an external framework.
#[repr(C)]
#[derive(Clone, Debug)]
pub struct DLManagedTensor {
    pub dl_tensor: DLTensor,
    pub manager_ctx: *mut std::ffi::c_void,
    pub deleter: Option<unsafe extern "C" fn(self_: *mut DLManagedTensor)>,
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

    #[test]
    fn test_pytorch_tensor_descriptor_contiguity_and_strides() {
        let desc = PyTorchTensorDescriptor::new_contiguous(
            0x2000,
            PyTorchDtype::Float32,
            PyTorchDevice::Cuda(0),
            vec![2, 3, 4],
            true,
        );

        assert_eq!(desc.numel(), 24);
        assert_eq!(desc.total_bytes(), 96);
        assert_eq!(desc.strides, vec![12, 4, 1]); // PyTorch element strides
        assert!(desc.is_contiguous());

        let buf = desc.to_buffer_descriptor();
        assert_eq!(buf.strides, vec![48, 16, 4]); // Byte strides
        assert!(buf.is_c_contiguous());
    }

    #[test]
    fn test_dlpack_layouts_and_sizes() {
        assert_eq!(std::mem::size_of::<DLDevice>(), 8);
        assert_eq!(std::mem::size_of::<DLDataType>(), 4);
    }
}
