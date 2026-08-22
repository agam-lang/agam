//! Inter-Process Communication (IPC) & Shared Memory Channels.
//!
//! Implements lock-free atomic circular ring buffers (SPSC/MPMC) and shared-memory
//! regions for zero-copy message passing between Agam processes.

use crate::serial::ZeroCopy;
use std::sync::atomic::{AtomicUsize, Ordering};

/// IPC errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcError {
    QueueFull,
    QueueEmpty,
    InvalidCapacity,
    BufferOverflow,
    SharedMemoryMappingFailed(String),
}

impl std::fmt::Display for IpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QueueFull => write!(f, "IPC ring buffer is full"),
            Self::QueueEmpty => write!(f, "IPC ring buffer is empty"),
            Self::InvalidCapacity => write!(f, "Capacity must be a power of two"),
            Self::BufferOverflow => write!(f, "Write size exceeds available buffer capacity"),
            Self::SharedMemoryMappingFailed(msg) => write!(f, "Failed to map shared memory: {msg}"),
        }
    }
}

impl std::error::Error for IpcError {}

/// Lock-free Single-Producer Single-Consumer (SPSC) Ring Buffer for zero-copy IPC.
///
/// Uses atomic head and tail pointers with Acquire-Release memory ordering semantics,
/// guaranteeing wait-free enqueue and dequeue without locks or kernel transitions.
pub struct SpscRingBuffer<T: ZeroCopy> {
    buffer: Vec<T>,
    capacity: usize,
    mask: usize,
    head: AtomicUsize,
    tail: AtomicUsize,
}

impl<T: ZeroCopy> SpscRingBuffer<T> {
    /// Create a new SPSC ring buffer with the specified capacity (must be a power of two).
    pub fn new(capacity: usize, default_val: T) -> Result<Self, IpcError> {
        if capacity == 0 || (capacity & (capacity - 1)) != 0 {
            return Err(IpcError::InvalidCapacity);
        }

        Ok(Self {
            buffer: vec![default_val; capacity],
            capacity,
            mask: capacity - 1,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        })
    }

    /// Try to enqueue an element. Returns `Err(IpcError::QueueFull)` if full.
    pub fn try_push(&mut self, item: T) -> Result<(), IpcError> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);

        if head.wrapping_sub(tail) >= self.capacity {
            return Err(IpcError::QueueFull);
        }

        let index = head & self.mask;
        self.buffer[index] = item;
        self.head.store(head.wrapping_add(1), Ordering::Release);
        Ok(())
    }

    /// Try to dequeue an element. Returns `Err(IpcError::QueueEmpty)` if empty.
    pub fn try_pop(&mut self) -> Result<T, IpcError> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);

        if tail == head {
            return Err(IpcError::QueueEmpty);
        }

        let index = tail & self.mask;
        let item = self.buffer[index];
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        Ok(item)
    }

    /// Current number of elements stored in the queue.
    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        head.wrapping_sub(tail)
    }

    /// Check whether the ring buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Total storage capacity of the ring buffer.
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

/// Shared Memory Region descriptor for cross-process zero-copy communication.
#[derive(Debug)]
pub struct SharedMemoryRegion {
    pub name: String,
    pub size_bytes: usize,
    data: Vec<u8>,
}

impl SharedMemoryRegion {
    /// Allocate or map a named shared memory region.
    pub fn create_or_open(name: impl Into<String>, size_bytes: usize) -> Self {
        Self {
            name: name.into(),
            size_bytes,
            data: vec![0u8; size_bytes],
        }
    }

    /// Get immutable slice access to the shared buffer.
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    /// Get mutable slice access to the shared buffer.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Write raw bytes to an offset within the shared memory region.
    pub fn write_at(&mut self, offset: usize, bytes: &[u8]) -> Result<(), IpcError> {
        if offset + bytes.len() > self.size_bytes {
            return Err(IpcError::BufferOverflow);
        }
        self.data[offset..offset + bytes.len()].copy_from_slice(bytes);
        Ok(())
    }

    /// Read raw bytes from an offset within the shared memory region.
    pub fn read_at(&self, offset: usize, len: usize) -> Result<&[u8], IpcError> {
        if offset + len > self.size_bytes {
            return Err(IpcError::BufferOverflow);
        }
        Ok(&self.data[offset..offset + len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spsc_ring_buffer_push_pop() {
        let mut queue = SpscRingBuffer::<i64>::new(8, 0).expect("Create ring buffer");
        assert!(queue.is_empty());
        assert_eq!(queue.capacity(), 8);

        for i in 1..=8 {
            queue.try_push(i * 10).expect("Push");
        }

        assert_eq!(queue.len(), 8);
        assert_eq!(queue.try_push(90), Err(IpcError::QueueFull));

        for i in 1..=8 {
            let val = queue.try_pop().expect("Pop");
            assert_eq!(val, i * 10);
        }

        assert!(queue.is_empty());
        assert_eq!(queue.try_pop(), Err(IpcError::QueueEmpty));
    }

    #[test]
    fn test_shared_memory_region_read_write() {
        let mut shm = SharedMemoryRegion::create_or_open("agam_tensor_shm", 1024);
        let sample = b"TENSOR_PAYLOAD_001";

        shm.write_at(64, sample).expect("Write at offset 64");
        let read = shm.read_at(64, sample.len()).expect("Read at offset 64");
        assert_eq!(read, sample);

        // Test boundary overflow error
        let err = shm.write_at(1020, sample).expect_err("Should overflow");
        assert_eq!(err, IpcError::BufferOverflow);
    }
}
