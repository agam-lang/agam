//! Snapshot Testing Infrastructure.
//!
//! Captures function outputs and verifies them against committed `.snap` files.
//! Supports automated diffing, interactive review, and batch snapshot updating.

use std::fs;
use std::path::PathBuf;

/// Error raised during snapshot assertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotError {
    Mismatch {
        name: String,
        expected: String,
        actual: String,
        diff: String,
    },
    IoError(String),
    NotFound(String),
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mismatch { name, diff, .. } => {
                write!(f, "Snapshot mismatch for `{name}`:\n{diff}")
            }
            Self::IoError(msg) => write!(f, "Snapshot I/O error: {msg}"),
            Self::NotFound(path) => write!(f, "Snapshot not found at `{path}`"),
        }
    }
}

impl std::error::Error for SnapshotError {}

/// Snapshot manager configuration and runner.
pub struct SnapshotManager {
    snapshot_dir: PathBuf,
    update_snapshots: bool,
}

impl SnapshotManager {
    pub fn new(snapshot_dir: impl Into<PathBuf>, update_snapshots: bool) -> Self {
        Self {
            snapshot_dir: snapshot_dir.into(),
            update_snapshots,
        }
    }

    /// Compute line-by-line unified diff between expected and actual strings.
    pub fn compute_diff(expected: &str, actual: &str) -> String {
        let exp_lines: Vec<&str> = expected.lines().collect();
        let act_lines: Vec<&str> = actual.lines().collect();

        let mut diff = String::new();
        let max_lines = exp_lines.len().max(act_lines.len());

        for i in 0..max_lines {
            match (exp_lines.get(i), act_lines.get(i)) {
                (Some(e), Some(a)) if e == a => {
                    diff.push_str(&format!("  {e}\n"));
                }
                (Some(e), Some(a)) => {
                    diff.push_str(&format!("- {e}\n"));
                    diff.push_str(&format!("+ {a}\n"));
                }
                (Some(e), None) => {
                    diff.push_str(&format!("- {e}\n"));
                }
                (None, Some(a)) => {
                    diff.push_str(&format!("+ {a}\n"));
                }
                (None, None) => {}
            }
        }
        diff
    }

    /// Assert that `actual` matches the snapshot with `name`.
    pub fn assert_snapshot(&self, name: &str, actual: &str) -> Result<(), SnapshotError> {
        let file_path = self.snapshot_dir.join(format!("{name}.snap"));

        if self.update_snapshots || !file_path.exists() {
            if let Some(parent) = file_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            fs::write(&file_path, actual).map_err(|e| SnapshotError::IoError(e.to_string()))?;
            return Ok(());
        }

        let expected =
            fs::read_to_string(&file_path).map_err(|e| SnapshotError::IoError(e.to_string()))?;

        if expected.trim() != actual.trim() {
            let diff = Self::compute_diff(&expected, actual);
            return Err(SnapshotError::Mismatch {
                name: name.to_string(),
                expected,
                actual: actual.to_string(),
                diff,
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_lifecycle_and_diffing() {
        let diff = SnapshotManager::compute_diff("hello\nworld", "hello\nagam");
        assert!(diff.contains("- world"));
        assert!(diff.contains("+ agam"));

        let temp_dir = std::env::temp_dir().join(format!("agam_snap_{}", std::process::id()));
        let mgr = SnapshotManager::new(&temp_dir, true);
        mgr.assert_snapshot("test_output", "output line 1\noutput line 2")
            .expect("Create snapshot");

        let verify_mgr = SnapshotManager::new(&temp_dir, false);
        assert!(
            verify_mgr
                .assert_snapshot("test_output", "output line 1\noutput line 2")
                .is_ok()
        );

        let mismatch = verify_mgr.assert_snapshot("test_output", "output line 1\noutput modified");
        assert!(mismatch.is_err());

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
