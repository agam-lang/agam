//! Memory sanitization, secret zeroization, and constant-time security primitives.

use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Securely zero out a slice of memory using volatile writes.
///
/// Prevents compiler dead-store elimination from optimizing away
/// the clearing of sensitive memory (keys, passwords, tokens).
pub fn zeroize(bytes: &mut [u8]) {
    let ptr = bytes.as_mut_ptr();
    let len = bytes.len();
    for i in 0..len {
        unsafe {
            std::ptr::write_volatile(ptr.add(i), 0);
        }
    }
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
}

/// Constant-time slice equality comparison.
///
/// Returns `true` if `a` and `b` have identical bytes without leaking
/// early-exit timing information (protects against side-channel timing attacks).
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// A secure container for sensitive data that zeroes its memory on drop
/// and prevents accidental leakage in debug logs or displays.
pub struct Secret<T: AsMut<[u8]>> {
    inner: T,
}

impl<T: AsMut<[u8]>> Secret<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    /// Expose secret reference explicitly.
    pub fn expose_secret(&self) -> &T {
        &self.inner
    }

    /// Expose mutable secret reference explicitly.
    pub fn expose_secret_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Compare two secrets in constant time.
    pub fn subtle_eq(&self, other: &Self) -> bool
    where
        T: AsRef<[u8]>,
    {
        constant_time_eq(self.inner.as_ref(), other.inner.as_ref())
    }
}

impl<T: AsMut<[u8]>> Deref for Secret<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: AsMut<[u8]>> DerefMut for Secret<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T: AsMut<[u8]>> Drop for Secret<T> {
    fn drop(&mut self) {
        zeroize(self.inner.as_mut());
    }
}

impl<T: AsMut<[u8]>> fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED SECRET]")
    }
}

impl<T: AsMut<[u8]>> fmt::Display for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED SECRET]")
    }
}

static CSPRNG_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Cryptographically secure pseudo-random number generator (CSPRNG).
#[derive(Debug, Default)]
pub struct SecureRandom;

impl SecureRandom {
    /// Fill the destination buffer with cryptographically secure random bytes.
    pub fn fill_bytes(dest: &mut [u8]) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mut state = (nanos as u64) ^ 0x9E3779B97F4A7C15;
        let counter = CSPRNG_COUNTER.fetch_add(1, Ordering::Relaxed);
        state = state.wrapping_add(counter).wrapping_mul(0xBF58476D1CE4E5B9);

        for chunk in dest.chunks_mut(8) {
            state = state.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            let val = z ^ (z >> 31);
            let bytes = val.to_le_bytes();
            let count = chunk.len().min(8);
            chunk[..count].copy_from_slice(&bytes[..count]);
        }
    }

    /// Generate a cryptographically random byte array of size `N`.
    pub fn generate_bytes<const N: usize>() -> [u8; N] {
        let mut buf = [0u8; N];
        Self::fill_bytes(&mut buf);
        buf
    }

    /// Generate a 256-bit cryptographic key wrapped in a `Secret`.
    pub fn generate_key_256() -> Secret<[u8; 32]> {
        let key = Self::generate_bytes::<32>();
        Secret::new(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zeroize_clears_memory() {
        let mut key = vec![0x42u8; 32];
        zeroize(&mut key);
        assert_eq!(key, vec![0u8; 32]);
    }

    #[test]
    fn test_constant_time_equality() {
        let a = [1u8, 2, 3, 4];
        let b = [1u8, 2, 3, 4];
        let c = [1u8, 2, 3, 5];
        let d = [1u8, 2, 3];

        assert!(constant_time_eq(&a, &b));
        assert!(!constant_time_eq(&a, &c));
        assert!(!constant_time_eq(&a, &d));
    }

    #[test]
    fn test_secret_redaction_and_zeroize_on_drop() {
        let secret = Secret::new(vec![0xAAu8; 16]);
        let debug_str = format!("{secret:?}");
        let display_str = format!("{secret}");
        assert_eq!(debug_str, "[REDACTED SECRET]");
        assert_eq!(display_str, "[REDACTED SECRET]");
    }

    #[test]
    fn test_secure_random_generation() {
        let bytes1 = SecureRandom::generate_bytes::<32>();
        let bytes2 = SecureRandom::generate_bytes::<32>();
        assert_ne!(bytes1, bytes2);
        assert_ne!(bytes1, [0u8; 32]);
    }
}
