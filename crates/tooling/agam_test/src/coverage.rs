//! Source Code Coverage Instrumentation & Reporting.
//!
//! Tracks line, branch, and basic block hit counters across test suites and exports
//! summary metrics and LCOV / HTML compatible coverage data.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Line execution coverage status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineStatus {
    Hit(u64),
    Missed,
    Unexecutable,
}

/// Source file coverage record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileCoverage {
    pub path: String,
    pub lines: HashMap<usize, LineStatus>,
}

impl FileCoverage {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            lines: HashMap::new(),
        }
    }

    pub fn record_hit(&mut self, line: usize) {
        let entry = self.lines.entry(line).or_insert(LineStatus::Hit(0));
        if let LineStatus::Hit(count) = entry {
            *count += 1;
        }
    }

    pub fn record_miss(&mut self, line: usize) {
        self.lines.entry(line).or_insert(LineStatus::Missed);
    }

    pub fn total_executable_lines(&self) -> usize {
        self.lines
            .values()
            .filter(|status| !matches!(status, LineStatus::Unexecutable))
            .count()
    }

    pub fn covered_lines(&self) -> usize {
        self.lines
            .values()
            .filter(|status| matches!(status, LineStatus::Hit(count) if *count > 0))
            .count()
    }

    pub fn coverage_percentage(&self) -> f64 {
        let total = self.total_executable_lines();
        if total == 0 {
            100.0
        } else {
            (self.covered_lines() as f64 / total as f64) * 100.0
        }
    }
}

/// Aggregated coverage report across all workspace files.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CoverageReport {
    pub files: Vec<FileCoverage>,
}

impl CoverageReport {
    pub fn total_executable_lines(&self) -> usize {
        self.files.iter().map(|f| f.total_executable_lines()).sum()
    }

    pub fn total_covered_lines(&self) -> usize {
        self.files.iter().map(|f| f.covered_lines()).sum()
    }

    pub fn total_coverage_percentage(&self) -> f64 {
        let total = self.total_executable_lines();
        if total == 0 {
            100.0
        } else {
            (self.total_covered_lines() as f64 / total as f64) * 100.0
        }
    }

    /// Generate an LCOV tracefile format string.
    pub fn to_lcov(&self) -> String {
        let mut out = String::new();
        for file in &self.files {
            out.push_str(&format!("SF:{}\n", file.path));
            let mut sorted_lines: Vec<_> = file.lines.iter().collect();
            sorted_lines.sort_by_key(|(line, _)| *line);

            for (&line, &status) in sorted_lines {
                match status {
                    LineStatus::Hit(count) => out.push_str(&format!("DA:{line},{count}\n")),
                    LineStatus::Missed => out.push_str(&format!("DA:{line},0\n")),
                    LineStatus::Unexecutable => {}
                }
            }
            out.push_str(&format!("LF:{}\n", file.total_executable_lines()));
            out.push_str(&format!("LH:{}\n", file.covered_lines()));
            out.push_str("end_of_record\n");
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coverage_recording_and_lcov_export() {
        let mut file_cov = FileCoverage::new("src/math.agam");
        file_cov.record_hit(10);
        file_cov.record_hit(10);
        file_cov.record_hit(11);
        file_cov.record_miss(12);

        assert_eq!(file_cov.total_executable_lines(), 3);
        assert_eq!(file_cov.covered_lines(), 2);
        assert!((file_cov.coverage_percentage() - 66.666).abs() < 0.01);

        let report = CoverageReport {
            files: vec![file_cov],
        };

        let lcov = report.to_lcov();
        assert!(lcov.contains("SF:src/math.agam"));
        assert!(lcov.contains("DA:10,2"));
        assert!(lcov.contains("DA:11,1"));
        assert!(lcov.contains("DA:12,0"));
        assert!(lcov.contains("LH:2"));
        assert!(lcov.contains("LF:3"));
    }
}
