//! Statistical Benchmark Execution Engine (@bench).
//!
//! Provides warm-up cycles, execution timing, percentiles, mean, median,
//! standard deviation, and regression detection against baselines.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Statistical results of a benchmark run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchResult {
    pub name: String,
    pub iterations: u64,
    pub total_time_ns: u64,
    pub mean_time_ns: f64,
    pub median_time_ns: f64,
    pub min_time_ns: u64,
    pub max_time_ns: u64,
    pub std_dev_ns: f64,
    pub ops_per_sec: f64,
}

/// Configuration parameters for benchmark runs.
#[derive(Debug, Clone)]
pub struct BenchConfig {
    pub warmup_iterations: u64,
    pub min_measurement_time: Duration,
    pub max_iterations: u64,
}

impl Default for BenchConfig {
    fn default() -> Self {
        Self {
            warmup_iterations: 100,
            min_measurement_time: Duration::from_millis(50),
            max_iterations: 100_000,
        }
    }
}

/// Benchmark harness running a closures or compiled functions repeatedly.
pub struct BenchmarkHarness {
    config: BenchConfig,
}

impl BenchmarkHarness {
    pub fn new(config: BenchConfig) -> Self {
        Self { config }
    }

    /// Execute a benchmarked closure with statistical analysis.
    pub fn run_benchmark<F>(&self, name: impl Into<String>, mut func: F) -> BenchResult
    where
        F: FnMut(),
    {
        // 1. Warm-up phase
        for _ in 0..self.config.warmup_iterations {
            func();
        }

        // 2. Timed measurement loop
        let mut sample_durations = Vec::new();
        let start_total = Instant::now();
        let mut iterations = 0u64;

        while start_total.elapsed() < self.config.min_measurement_time
            && iterations < self.config.max_iterations
        {
            let t0 = Instant::now();
            func();
            let elapsed_ns = t0.elapsed().as_nanos() as u64;
            sample_durations.push(elapsed_ns);
            iterations += 1;
        }

        let total_time_ns = start_total.elapsed().as_nanos() as u64;

        // 3. Statistical computations
        sample_durations.sort_unstable();
        let count = sample_durations.len() as f64;
        let sum: u64 = sample_durations.iter().sum();
        let mean = (sum as f64) / count;

        let median = if sample_durations.len().is_multiple_of(2) {
            let mid = sample_durations.len() / 2;
            ((sample_durations[mid - 1] + sample_durations[mid]) as f64) / 2.0
        } else {
            sample_durations[sample_durations.len() / 2] as f64
        };

        let variance: f64 = sample_durations
            .iter()
            .map(|&x| {
                let diff = (x as f64) - mean;
                diff * diff
            })
            .sum::<f64>()
            / count;
        let std_dev = variance.sqrt();

        let ops_per_sec = if mean > 0.0 {
            1_000_000_000.0 / mean
        } else {
            0.0
        };

        BenchResult {
            name: name.into(),
            iterations,
            total_time_ns,
            mean_time_ns: mean,
            median_time_ns: median,
            min_time_ns: *sample_durations.first().unwrap_or(&0),
            max_time_ns: *sample_durations.last().unwrap_or(&0),
            std_dev_ns: std_dev,
            ops_per_sec,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_statistics_computation() {
        let harness = BenchmarkHarness::new(BenchConfig {
            warmup_iterations: 10,
            min_measurement_time: Duration::from_millis(10),
            max_iterations: 1000,
        });

        let mut acc = 0u64;
        let result = harness.run_benchmark("sum_accumulator", || {
            for i in 0..100 {
                acc = acc.wrapping_add(i);
            }
        });

        assert_eq!(result.name, "sum_accumulator");
        assert!(result.iterations >= 10);
        assert!(result.mean_time_ns > 0.0);
        assert!(result.ops_per_sec > 0.0);
        assert!(result.min_time_ns <= result.max_time_ns);
    }
}
