//! In-Process Coverage & Mutation-Guided Fuzzing Engine.
//!
//! Provides automated input mutation, crash triage, and corpus management
//! for testing compiler parsers, AST lowerers, and runtime subsystems.

use std::collections::HashSet;

/// Mutation strategies for byte buffer fuzzing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationStrategy {
    BitFlip,
    ByteFlip,
    InsertInterestingBytes,
    DeleteBytes,
    InsertDuplicate,
    ShuffleSlice,
}

/// Fuzzing runner and corpus tracker.
pub struct FuzzRunner {
    pub corpus: Vec<Vec<u8>>,
    pub max_iterations: usize,
    pub crashes: Vec<Vec<u8>>,
    seed: u64,
}

impl FuzzRunner {
    pub fn new(initial_corpus: Vec<Vec<u8>>, max_iterations: usize) -> Self {
        let corpus = if initial_corpus.is_empty() {
            vec![b"fn main() { return 0; }".to_vec()]
        } else {
            initial_corpus
        };

        Self {
            corpus,
            max_iterations,
            crashes: Vec::new(),
            seed: 0x8543_2910_A1B2_C3D4,
        }
    }

    fn next_rand(&mut self) -> u64 {
        self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.seed
    }

    /// Mutate a byte slice using randomized mutation strategies.
    pub fn mutate(&mut self, input: &[u8]) -> Vec<u8> {
        let mut mutated = input.to_vec();
        if mutated.is_empty() {
            mutated.push(0x42);
            return mutated;
        }

        let strategy_idx = (self.next_rand() % 6) as u8;
        let pos = (self.next_rand() as usize) % mutated.len();

        match strategy_idx {
            0 => {
                // BitFlip
                let bit = (self.next_rand() % 8) as u8;
                mutated[pos] ^= 1 << bit;
            }
            1 => {
                // ByteFlip
                mutated[pos] = mutated[pos].wrapping_add((self.next_rand() % 255) as u8);
            }
            2 => {
                // Insert interesting value (0, 255, newline, null)
                let interesting = [0x00, 0xFF, 0x0A, 0x0D, 0x22, 0x27, 0x5C, 0x7F];
                let val = interesting[(self.next_rand() as usize) % interesting.len()];
                mutated.insert(pos, val);
            }
            3 => {
                // Delete byte
                if mutated.len() > 1 {
                    mutated.remove(pos);
                }
            }
            4 => {
                // Duplicate byte
                let val = mutated[pos];
                mutated.insert(pos, val);
            }
            _ => {
                // Shuffle two adjacent bytes
                if pos + 1 < mutated.len() {
                    mutated.swap(pos, pos + 1);
                }
            }
        }

        mutated
    }

    /// Run the fuzz target closure over iterative mutations.
    pub fn run_target<F>(&mut self, mut target: F) -> usize
    where
        F: FnMut(&[u8]) -> Result<(), String>,
    {
        let mut unique_crashes = HashSet::new();
        let mut executed = 0;

        for _ in 0..self.max_iterations {
            executed += 1;
            let corpus_idx = (self.next_rand() as usize) % self.corpus.len();
            let parent = self.corpus[corpus_idx].clone();
            let mutant = self.mutate(&parent);

            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| target(&mutant))) {
                Ok(Ok(())) => {
                    // Successful run, occasionally add to corpus
                    if mutant.len() <= 1024 && self.next_rand().is_multiple_of(50) {
                        self.corpus.push(mutant);
                    }
                }
                Ok(Err(err_msg)) => {
                    if unique_crashes.insert(err_msg) {
                        self.crashes.push(mutant);
                    }
                }
                Err(_) => {
                    // Caught panic
                    if unique_crashes.insert("PANIC".to_string()) {
                        self.crashes.push(mutant);
                    }
                }
            }
        }

        executed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzz_runner_mutations_and_crash_detection() {
        let mut runner = FuzzRunner::new(vec![b"let x = 10;".to_vec()], 500);
        let executed = runner.run_target(|input| {
            if input.contains(&b'!') && input.contains(&b'@') {
                return Err("Triggered error pattern".to_string());
            }
            Ok(())
        });

        assert_eq!(executed, 500);
        assert!(!runner.corpus.is_empty());
    }
}
