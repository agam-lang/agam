//! First-party file and path I/O helpers.
//!
//! This is the smallest standard-library I/O slice: deterministic text-file
//! operations and path inspection utilities that higher-level effects work can
//! later wrap instead of reinventing.

use std::fmt;
use std::path::{Path, PathBuf};

/// Structured I/O failure carrying the operation and path context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IoError {
    pub operation: &'static str,
    pub path: PathBuf,
    pub message: String,
}

impl IoError {
    fn new(operation: &'static str, path: &Path, error: impl fmt::Display) -> Self {
        Self {
            operation,
            path: path.to_path_buf(),
            message: error.to_string(),
        }
    }
}

impl fmt::Display for IoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} `{}` failed: {}",
            self.operation,
            self.path.display(),
            self.message
        )
    }
}

impl std::error::Error for IoError {}

/// Return whether a filesystem path exists.
pub fn exists(path: impl AsRef<Path>) -> bool {
    path.as_ref().exists()
}

/// Return whether a filesystem path is a regular file.
pub fn is_file(path: impl AsRef<Path>) -> bool {
    path.as_ref().is_file()
}

/// Return whether a filesystem path is a directory.
pub fn is_dir(path: impl AsRef<Path>) -> bool {
    path.as_ref().is_dir()
}

/// Create a directory and all of its parents.
pub fn create_dir_all(path: impl AsRef<Path>) -> Result<(), IoError> {
    let path = path.as_ref();
    std::fs::create_dir_all(path).map_err(|error| IoError::new("create_dir_all", path, error))
}

/// Read one UTF-8 text file into memory.
pub fn read_to_string(path: impl AsRef<Path>) -> Result<String, IoError> {
    let path = path.as_ref();
    std::fs::read_to_string(path).map_err(|error| IoError::new("read_to_string", path, error))
}

/// Read one UTF-8 text file and split it into owned lines.
pub fn read_lines(path: impl AsRef<Path>) -> Result<Vec<String>, IoError> {
    let text = read_to_string(path)?;
    Ok(text.lines().map(str::to_string).collect())
}

/// Write a UTF-8 text file, creating parent directories when needed.
pub fn write_string(path: impl AsRef<Path>, contents: impl AsRef<str>) -> Result<(), IoError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            create_dir_all(parent)?;
        }
    }
    std::fs::write(path, contents.as_ref())
        .map_err(|error| IoError::new("write_string", path, error))
}

/// Append UTF-8 text to a file, creating parent directories when needed.
pub fn append_string(path: impl AsRef<Path>, contents: impl AsRef<str>) -> Result<(), IoError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            create_dir_all(parent)?;
        }
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| IoError::new("append_string", path, error))?;
    use std::io::Write as _;
    file.write_all(contents.as_ref().as_bytes())
        .map_err(|error| IoError::new("append_string", path, error))
}

/// List one directory deterministically in lexicographic path order.
pub fn list_dir(path: impl AsRef<Path>) -> Result<Vec<PathBuf>, IoError> {
    let path = path.as_ref();
    let mut entries = std::fs::read_dir(path)
        .map_err(|error| IoError::new("list_dir", path, error))?
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|error| IoError::new("list_dir", path, error))
        })
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort();
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("agam_std_io_{label}_{stamp}"));
        std::fs::create_dir_all(&path).expect("temp dir should be created");
        path
    }

    #[test]
    fn write_and_read_text_round_trip() {
        let root = temp_dir("round_trip");
        let file = root.join("nested").join("demo.txt");

        write_string(&file, "hello\nagam\n").expect("write should succeed");
        let text = read_to_string(&file).expect("read should succeed");

        assert_eq!(text, "hello\nagam\n");
        assert!(exists(&file));
        assert!(is_file(&file));
        assert!(is_dir(root.join("nested")));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn append_and_read_lines_preserve_order() {
        let root = temp_dir("append_lines");
        let file = root.join("demo.txt");

        append_string(&file, "alpha\n").expect("first append should succeed");
        append_string(&file, "beta\n").expect("second append should succeed");

        let lines = read_lines(&file).expect("read lines should succeed");
        assert_eq!(lines, vec!["alpha".to_string(), "beta".to_string()]);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn list_dir_is_sorted() {
        let root = temp_dir("list_sorted");
        let zeta = root.join("zeta.txt");
        let alpha = root.join("alpha.txt");
        let beta = root.join("beta.txt");

        write_string(&zeta, "z").expect("write zeta");
        write_string(&alpha, "a").expect("write alpha");
        write_string(&beta, "b").expect("write beta");

        let entries = list_dir(&root).expect("list dir should succeed");
        let names = entries
            .iter()
            .map(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .expect("file name should be utf-8")
                    .to_string()
            })
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["alpha.txt", "beta.txt", "zeta.txt"]);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn missing_file_reports_operation_and_path() {
        let root = temp_dir("missing");
        let file = root.join("missing.txt");

        let error = read_to_string(&file).expect_err("missing file should fail");
        assert_eq!(error.operation, "read_to_string");
        assert_eq!(error.path, file);
        assert!(error.to_string().contains("read_to_string"));

        let _ = std::fs::remove_dir_all(root);
    }
}
