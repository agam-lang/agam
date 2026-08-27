//! First-party statistical PRNG and random sampling utilities powered by `rand`.
//!
//! Exposes thread-local entropy sources and seedable deterministic generators
//! per `ADOPTED_DEPENDENCIES.md` and `note.md`.

#![deny(clippy::unwrap_used)]

use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng as _, SeedableRng};

/// Generate a random integer in the half-open range `[low, high)`.
pub fn int_range(low: i64, high: i64) -> i64 {
    if low >= high {
        return low;
    }
    let mut rng = rand::thread_rng();
    rng.gen_range(low..high)
}

/// Generate a uniform random float in `[0.0, 1.0)`.
pub fn float() -> f64 {
    let mut rng = rand::thread_rng();
    rng.gen_range(0.0..1.0)
}

/// Randomly select one element from a slice with uniform distribution.
pub fn choice<'a, T>(slice: &'a [T]) -> Option<&'a T> {
    let mut rng = rand::thread_rng();
    slice.choose(&mut rng)
}

/// Randomly shuffle a slice in-place using the Fisher-Yates algorithm.
pub fn shuffle<T>(slice: &mut [T]) {
    let mut rng = rand::thread_rng();
    slice.shuffle(&mut rng);
}

/// Seedable, deterministic pseudo-random number generator.
#[derive(Clone, Debug)]
pub struct Rng {
    inner: StdRng,
}

impl Rng {
    /// Create a new deterministic generator seeded with the provided `u64`.
    pub fn with_seed(seed: u64) -> Self {
        Self {
            inner: StdRng::seed_from_u64(seed),
        }
    }

    /// Generate a random integer in the half-open range `[low, high)`.
    pub fn gen_int_range(&mut self, low: i64, high: i64) -> i64 {
        if low >= high {
            return low;
        }
        self.inner.gen_range(low..high)
    }

    /// Generate a uniform random float in `[0.0, 1.0)`.
    pub fn gen_float(&mut self) -> f64 {
        self.inner.gen_range(0.0..1.0)
    }

    /// Randomly select one element from a slice with uniform distribution.
    pub fn gen_choice<'a, T>(&mut self, slice: &'a [T]) -> Option<&'a T> {
        slice.choose(&mut self.inner)
    }

    /// Randomly shuffle a slice in-place.
    pub fn gen_shuffle<T>(&mut self, slice: &mut [T]) {
        slice.shuffle(&mut self.inner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_int_range_within_bounds() {
        for _ in 0..100 {
            let n = int_range(10, 20);
            assert!(n >= 10 && n < 20);
        }
        assert_eq!(int_range(5, 5), 5);
    }

    #[test]
    fn test_float_within_unit_interval() {
        for _ in 0..100 {
            let f = float();
            assert!(f >= 0.0 && f < 1.0);
        }
    }

    #[test]
    fn test_choice_and_shuffle() {
        let items = vec![1, 2, 3, 4, 5];
        let chosen = choice(&items);
        assert!(chosen.is_some());
        if let Some(&c) = chosen {
            assert!(items.contains(&c));
        }

        let mut shuffled = items.clone();
        shuffle(&mut shuffled);
        assert_eq!(shuffled.len(), 5);
        for item in &items {
            assert!(shuffled.contains(item));
        }
    }

    #[test]
    fn test_deterministic_rng_seed() {
        let mut rng1 = Rng::with_seed(42);
        let mut rng2 = Rng::with_seed(42);

        let seq1: Vec<i64> = (0..10).map(|_| rng1.gen_int_range(0, 1000)).collect();
        let seq2: Vec<i64> = (0..10).map(|_| rng2.gen_int_range(0, 1000)).collect();

        assert_eq!(seq1, seq2, "Same seed must produce identical PRNG stream");
    }
}
