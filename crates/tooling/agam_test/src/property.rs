//! Property-Based Random Testing Engine (@test.property).
//!
//! Generates random test inputs, executes invariant predicates, and shrinks failing
//! cases to minimal reproducing counterexamples.

/// Pseudorandom generator with seed repeatability (PCG/XorShift64*).
pub struct TestRng {
    state: u64,
}

impl TestRng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0x853c49e6748fea9b } else { seed },
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    pub fn gen_i64_range(&mut self, min: i64, max: i64) -> i64 {
        if min >= max {
            return min;
        }
        let range = (max - min) as u64;
        let val = self.next_u64() % (range + 1);
        min + (val as i64)
    }

    pub fn gen_vec_i64(&mut self, max_len: usize, min_val: i64, max_val: i64) -> Vec<i64> {
        let len = (self.next_u64() as usize) % (max_len + 1);
        (0..len)
            .map(|_| self.gen_i64_range(min_val, max_val))
            .collect()
    }
}

/// Property test result indicating success or failure with counterexample.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyResult<T> {
    Passed {
        tests_run: usize,
    },
    Failed {
        tests_run: usize,
        counterexample: T,
        shrunk_counterexample: T,
    },
}

/// Property test runner.
pub struct PropertyRunner {
    pub num_trials: usize,
    pub seed: u64,
}

impl Default for PropertyRunner {
    fn default() -> Self {
        Self {
            num_trials: 100,
            seed: 42,
        }
    }
}

impl PropertyRunner {
    pub fn new(num_trials: usize, seed: u64) -> Self {
        Self { num_trials, seed }
    }

    /// Check an integer property invariant $P(x)$ with shrinking.
    pub fn check_i64<P>(&self, min: i64, max: i64, predicate: P) -> PropertyResult<i64>
    where
        P: Fn(i64) -> bool,
    {
        let mut rng = TestRng::new(self.seed);

        for trial in 1..=self.num_trials {
            let sample = rng.gen_i64_range(min, max);
            if !predicate(sample) {
                // Shrink towards zero
                let mut shrunk = sample;
                let step = if sample > 0 { -1 } else { 1 };
                let mut candidate = sample;

                while candidate != 0 {
                    candidate += step;
                    if !predicate(candidate) {
                        shrunk = candidate;
                    } else {
                        break;
                    }
                }

                return PropertyResult::Failed {
                    tests_run: trial,
                    counterexample: sample,
                    shrunk_counterexample: shrunk,
                };
            }
        }

        PropertyResult::Passed {
            tests_run: self.num_trials,
        }
    }

    /// Check a vector slice property invariant $P(\vec{x})$ with element and length shrinking.
    pub fn check_vec_i64<P>(&self, max_len: usize, predicate: P) -> PropertyResult<Vec<i64>>
    where
        P: Fn(&[i64]) -> bool,
    {
        let mut rng = TestRng::new(self.seed);

        for trial in 1..=self.num_trials {
            let sample = rng.gen_vec_i64(max_len, -1000, 1000);
            if !predicate(&sample) {
                // Shrink by removing elements from the end
                let mut shrunk = sample.clone();
                while shrunk.len() > 1 {
                    let mut smaller = shrunk.clone();
                    smaller.pop();
                    if !predicate(&smaller) {
                        shrunk = smaller;
                    } else {
                        break;
                    }
                }

                return PropertyResult::Failed {
                    tests_run: trial,
                    counterexample: sample,
                    shrunk_counterexample: shrunk,
                };
            }
        }

        PropertyResult::Passed {
            tests_run: self.num_trials,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_commutative_addition() {
        let runner = PropertyRunner::default();
        let result = runner.check_i64(-1000, 1000, |x| {
            let y = 42;
            x + y == y + x
        });
        assert_eq!(result, PropertyResult::Passed { tests_run: 100 });
    }

    #[test]
    fn test_property_failure_and_shrinking() {
        let runner = PropertyRunner::default();
        // Fails if x >= 50
        let result = runner.check_i64(0, 1000, |x| x < 50);

        match result {
            PropertyResult::Failed {
                tests_run,
                counterexample,
                shrunk_counterexample,
            } => {
                assert!(tests_run > 0);
                assert!(counterexample >= 50);
                assert_eq!(shrunk_counterexample, 50); // Shrunk to minimal failing value
            }
            PropertyResult::Passed { .. } => panic!("Expected property failure"),
        }
    }
}
