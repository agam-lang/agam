//! Atomic Reference Counting (ARC) runtime for Agam.
//!
//! The default memory management mode. Values are refcounted — no borrow
//! checker, no use-after-move. When the last reference goes out of scope,
//! the value is deallocated.
//!
//! ## Design
//! - Thread-safe via atomic operations (like Rust's `Arc`).
//! - Cycle detection via weak references (`AgamWeak`).
//! - Zero overhead in `strict` blocks (ARC is bypassed).

use std::sync::atomic::{AtomicU32, Ordering};

/// The ARC control block — stored alongside each heap-allocated value.
#[repr(C)]
pub struct ArcHeader {
    /// Strong reference count.
    strong: AtomicU32,
    /// Weak reference count.
    weak: AtomicU32,
}

impl ArcHeader {
    /// Create a new ARC header with refcount = 1.
    pub fn new() -> Self {
        Self {
            strong: AtomicU32::new(1),
            weak: AtomicU32::new(0),
        }
    }

    /// Increment the strong reference count. Returns the new count.
    pub fn retain(&self) -> u32 {
        self.strong.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Decrement the strong reference count. Returns the new count.
    /// If this returns 0, the caller should deallocate the value.
    pub fn release(&self) -> u32 {
        let prev = self.strong.fetch_sub(1, Ordering::Release);
        if prev == 1 {
            // Ensure all writes to the value are visible before deallocation.
            std::sync::atomic::fence(Ordering::Acquire);
        }
        prev - 1
    }

    /// Get the current strong count.
    pub fn strong_count(&self) -> u32 {
        self.strong.load(Ordering::Relaxed)
    }

    /// Get the current weak count.
    pub fn weak_count(&self) -> u32 {
        self.weak.load(Ordering::Relaxed)
    }

    /// Add a weak reference.
    pub fn weak_retain(&self) -> u32 {
        self.weak.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Remove a weak reference.
    pub fn weak_release(&self) -> u32 {
        self.weak.fetch_sub(1, Ordering::Release) - 1
    }
}

/// A managed ARC pointer — owns a value with automatic reference counting.
///
/// This is the runtime representation of all heap values in ARC mode.
pub struct AgamArc<T> {
    header: Box<ArcHeader>,
    value: Box<T>,
}

impl<T> AgamArc<T> {
    /// Create a new ARC-managed value.
    pub fn new(value: T) -> Self {
        Self {
            header: Box::new(ArcHeader::new()),
            value: Box::new(value),
        }
    }

    /// Get a reference to the managed value.
    pub fn get(&self) -> &T {
        &self.value
    }

    /// Get the current strong reference count.
    pub fn strong_count(&self) -> u32 {
        self.header.strong_count()
    }

    /// Retain (increment refcount).
    pub fn retain(&self) -> u32 {
        self.header.retain()
    }

    /// Release (decrement refcount). Returns true if this was the last reference.
    pub fn release(&self) -> bool {
        self.header.release() == 0
    }
}

/// A weak reference — does not prevent deallocation.
pub struct AgamWeak {
    header: *const ArcHeader,
}

impl AgamWeak {
    /// Try to upgrade to a strong reference. Returns false if the value was dropped.
    pub fn is_alive(&self) -> bool {
        unsafe {
            if self.header.is_null() { return false; }
            (*self.header).strong_count() > 0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arc_header_new() {
        let h = ArcHeader::new();
        assert_eq!(h.strong_count(), 1);
        assert_eq!(h.weak_count(), 0);
    }

    #[test]
    fn test_arc_retain_release() {
        let h = ArcHeader::new();
        assert_eq!(h.retain(), 2);
        assert_eq!(h.retain(), 3);
        assert_eq!(h.strong_count(), 3);
        assert_eq!(h.release(), 2);
        assert_eq!(h.release(), 1);
        assert_eq!(h.release(), 0); // should deallocate
    }

    #[test]
    fn test_arc_weak() {
        let h = ArcHeader::new();
        assert_eq!(h.weak_retain(), 1);
        assert_eq!(h.weak_count(), 1);
        assert_eq!(h.weak_release(), 0);
    }

    #[test]
    fn test_agam_arc_value() {
        let arc = AgamArc::new(42i64);
        assert_eq!(*arc.get(), 42);
        assert_eq!(arc.strong_count(), 1);
    }

    #[test]
    fn test_agam_arc_retain_release() {
        let arc = AgamArc::new(100i64);
        assert_eq!(arc.retain(), 2);
        assert_eq!(arc.strong_count(), 2);
        assert!(!arc.release()); // refcount = 1, not last
        assert!(arc.release()); // refcount = 0, last reference
    }

    #[test]
    fn test_agam_arc_string() {
        let arc = AgamArc::new(String::from("hello agam"));
        assert_eq!(arc.get().as_str(), "hello agam");
        arc.retain();
        assert_eq!(arc.strong_count(), 2);
    }
}
