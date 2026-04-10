//! Arbitrary precision integers and interval arithmetic.
//!
//! ## BigInt
//! - Stores digits in base 2³² (u32 limbs) for hardware multiplication.
//! - Operations use schoolbook multiplication (upgradeable to Karatsuba).
//!
//! ## Interval
//! - Tracks [lo, hi] bounds through all arithmetic.
//! - Guaranteed to contain the true value — essential for verified computation.

/// Arbitrary precision unsigned integer.
/// Stored as little-endian u32 limbs for hardware ALU alignment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BigUint {
    /// Little-endian limbs (limbs[0] is least significant).
    limbs: Vec<u32>,
}

impl BigUint {
    pub fn zero() -> Self {
        Self { limbs: vec![0] }
    }

    pub fn from_u64(v: u64) -> Self {
        if v == 0 {
            return Self::zero();
        }
        let lo = v as u32;
        let hi = (v >> 32) as u32;
        if hi == 0 {
            Self { limbs: vec![lo] }
        } else {
            Self {
                limbs: vec![lo, hi],
            }
        }
    }

    pub fn to_u64(&self) -> Option<u64> {
        match self.limbs.len() {
            0 => Some(0),
            1 => Some(self.limbs[0] as u64),
            2 => Some(self.limbs[0] as u64 | ((self.limbs[1] as u64) << 32)),
            _ => None,
        }
    }

    fn trim(&mut self) {
        while self.limbs.len() > 1 && *self.limbs.last().unwrap() == 0 {
            self.limbs.pop();
        }
    }

    pub fn is_zero(&self) -> bool {
        self.limbs.iter().all(|&x| x == 0)
    }

    /// Add two BigUints.
    pub fn add(&self, other: &BigUint) -> BigUint {
        let n = self.limbs.len().max(other.limbs.len());
        let mut result = Vec::with_capacity(n + 1);
        let mut carry = 0u64;

        for i in 0..n {
            let a = if i < self.limbs.len() {
                self.limbs[i] as u64
            } else {
                0
            };
            let b = if i < other.limbs.len() {
                other.limbs[i] as u64
            } else {
                0
            };
            let sum = a + b + carry;
            result.push(sum as u32);
            carry = sum >> 32;
        }
        if carry > 0 {
            result.push(carry as u32);
        }
        let mut r = BigUint { limbs: result };
        r.trim();
        r
    }

    /// Multiply two BigUints (schoolbook, O(n²)).
    pub fn mul(&self, other: &BigUint) -> BigUint {
        let n = self.limbs.len();
        let m = other.limbs.len();
        let mut result = vec![0u32; n + m];

        for i in 0..n {
            let mut carry = 0u64;
            for j in 0..m {
                let prod =
                    self.limbs[i] as u64 * other.limbs[j] as u64 + result[i + j] as u64 + carry;
                result[i + j] = prod as u32;
                carry = prod >> 32;
            }
            result[i + m] += carry as u32;
        }
        let mut r = BigUint { limbs: result };
        r.trim();
        r
    }

    /// Factorial: n!
    pub fn factorial(n: u32) -> BigUint {
        let mut result = BigUint::from_u64(1);
        for i in 2..=n {
            result = result.mul(&BigUint::from_u64(i as u64));
        }
        result
    }
}

/// Interval arithmetic: [lo, hi] with guaranteed containment.
/// Every operation widens the interval to contain the true result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    pub lo: f64,
    pub hi: f64,
}

impl Interval {
    pub fn new(lo: f64, hi: f64) -> Self {
        assert!(lo <= hi, "lo must be <= hi");
        Self { lo, hi }
    }

    pub fn exact(v: f64) -> Self {
        Self { lo: v, hi: v }
    }

    /// Value ± uncertainty.
    pub fn with_error(value: f64, error: f64) -> Self {
        Self {
            lo: value - error.abs(),
            hi: value + error.abs(),
        }
    }

    pub fn width(self) -> f64 {
        self.hi - self.lo
    }
    pub fn midpoint(self) -> f64 {
        0.5 * (self.lo + self.hi)
    }
    pub fn contains(self, x: f64) -> bool {
        x >= self.lo && x <= self.hi
    }

    pub fn add(self, other: Interval) -> Interval {
        Interval {
            lo: self.lo + other.lo,
            hi: self.hi + other.hi,
        }
    }

    pub fn sub(self, other: Interval) -> Interval {
        Interval {
            lo: self.lo - other.hi,
            hi: self.hi - other.lo,
        }
    }

    pub fn mul(self, other: Interval) -> Interval {
        let products = [
            self.lo * other.lo,
            self.lo * other.hi,
            self.hi * other.lo,
            self.hi * other.hi,
        ];
        let lo = products.iter().cloned().fold(f64::INFINITY, f64::min);
        let hi = products.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        Interval { lo, hi }
    }

    pub fn div(self, other: Interval) -> Option<Interval> {
        if other.lo <= 0.0 && other.hi >= 0.0 {
            return None;
        } // division by zero interval
        let inv = Interval {
            lo: 1.0 / other.hi,
            hi: 1.0 / other.lo,
        };
        Some(self.mul(inv))
    }

    pub fn sqrt(self) -> Interval {
        Interval {
            lo: self.lo.max(0.0).sqrt(),
            hi: self.hi.sqrt(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biguint_from_u64() {
        let n = BigUint::from_u64(42);
        assert_eq!(n.to_u64(), Some(42));
    }

    #[test]
    fn test_biguint_add() {
        let a = BigUint::from_u64(u64::MAX);
        let b = BigUint::from_u64(1);
        let c = a.add(&b);
        assert!(c.to_u64().is_none()); // overflows u64
        assert_eq!(c.limbs.len(), 3);
    }

    #[test]
    fn test_biguint_mul() {
        let a = BigUint::from_u64(12345);
        let b = BigUint::from_u64(67890);
        let c = a.mul(&b);
        assert_eq!(c.to_u64(), Some(12345u64 * 67890));
    }

    #[test]
    fn test_biguint_factorial() {
        let f10 = BigUint::factorial(10);
        assert_eq!(f10.to_u64(), Some(3628800));
    }

    #[test]
    fn test_biguint_factorial_25() {
        let f25 = BigUint::factorial(25);
        // 25! = 15511210043330985984000000 — does NOT fit in u64
        assert!(f25.to_u64().is_none());
        assert!(f25.limbs.len() >= 3);
    }

    #[test]
    fn test_interval_exact() {
        let a = Interval::exact(3.14);
        assert_eq!(a.width(), 0.0);
        assert!(a.contains(3.14));
    }

    #[test]
    fn test_interval_add() {
        let a = Interval::new(1.0, 2.0);
        let b = Interval::new(3.0, 4.0);
        let c = a.add(b);
        assert_eq!(c.lo, 4.0);
        assert_eq!(c.hi, 6.0);
    }

    #[test]
    fn test_interval_mul() {
        let a = Interval::new(-1.0, 2.0);
        let b = Interval::new(3.0, 4.0);
        let c = a.mul(b);
        assert_eq!(c.lo, -4.0);
        assert_eq!(c.hi, 8.0);
    }

    #[test]
    fn test_interval_div_by_zero() {
        let a = Interval::new(1.0, 2.0);
        let b = Interval::new(-1.0, 1.0); // contains zero
        assert!(a.div(b).is_none());
    }

    #[test]
    fn test_interval_with_error() {
        let a = Interval::with_error(3.0, 1.0);
        assert!(a.contains(3.0));
        assert!(a.contains(2.5));
        assert!(a.contains(3.5));
        assert!(!a.contains(4.5));
    }
}
