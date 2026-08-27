//! First-party file, path, and buffered stream I/O helpers.
//!
//! Provides deterministic text/binary file operations, 8KB chunk-buffered streams,
//! and object-oriented path utilities per `note.md`.

#![deny(clippy::unwrap_used)]

use std::fmt;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::ops::Div;
use std::path::{Path, PathBuf};

/// Structured I/O failure carrying the operation and path context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IoError {
    pub operation: &'static str,
    pub path: PathBuf,
    pub message: String,
}

impl IoError {
    pub fn new(operation: &'static str, path: &Path, error: impl fmt::Display) -> Self {
        Self {
            operation,
            path: path.to_path_buf(),
            message: error.to_string(),
        }
    }

    pub fn generic(operation: &'static str, message: impl fmt::Display) -> Self {
        Self {
            operation,
            path: PathBuf::new(),
            message: message.to_string(),
        }
    }
}

impl fmt::Display for IoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.as_os_str().is_empty() {
            write!(f, "{} failed: {}", self.operation, self.message)
        } else {
            write!(
                f,
                "{} `{}` failed: {}",
                self.operation,
                self.path.display(),
                self.message
            )
        }
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

/// High-throughput chunk-buffered stream reader (default 8KB).
pub struct FastBufReader<R: Read> {
    reader: BufReader<R>,
}

impl<R: Read> FastBufReader<R> {
    pub const DEFAULT_CAPACITY: usize = 8192;

    pub fn new(inner: R) -> Self {
        Self {
            reader: BufReader::with_capacity(Self::DEFAULT_CAPACITY, inner),
        }
    }

    pub fn with_capacity(capacity: usize, inner: R) -> Self {
        Self {
            reader: BufReader::with_capacity(capacity, inner),
        }
    }

    pub fn read_line(&mut self) -> Result<Option<String>, IoError> {
        let mut line = String::new();
        match self.reader.read_line(&mut line) {
            Ok(0) => Ok(None),
            Ok(_) => Ok(Some(line)),
            Err(e) => Err(IoError::generic("read_line", e)),
        }
    }

    pub fn read_all(&mut self) -> Result<Vec<u8>, IoError> {
        let mut buf = Vec::new();
        self.reader
            .read_to_end(&mut buf)
            .map_err(|e| IoError::generic("read_all", e))?;
        Ok(buf)
    }
}

/// High-throughput chunk-buffered stream writer (default 8KB).
pub struct FastBufWriter<W: Write> {
    writer: BufWriter<W>,
}

impl<W: Write> FastBufWriter<W> {
    pub const DEFAULT_CAPACITY: usize = 8192;

    pub fn new(inner: W) -> Self {
        Self {
            writer: BufWriter::with_capacity(Self::DEFAULT_CAPACITY, inner),
        }
    }

    pub fn with_capacity(capacity: usize, inner: W) -> Self {
        Self {
            writer: BufWriter::with_capacity(capacity, inner),
        }
    }

    pub fn write_str(&mut self, s: &str) -> Result<(), IoError> {
        self.writer
            .write_all(s.as_bytes())
            .map_err(|e| IoError::generic("write_str", e))
    }

    pub fn write_line(&mut self, s: &str) -> Result<(), IoError> {
        self.write_str(s)?;
        self.writer
            .write_all(b"\n")
            .map_err(|e| IoError::generic("write_line", e))
    }

    pub fn flush(&mut self) -> Result<(), IoError> {
        self.writer
            .flush()
            .map_err(|e| IoError::generic("flush", e))
    }
}

/// Object-oriented path builder with fluent operator overloads (`note.md`).
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AgamPath {
    inner: PathBuf,
}

impl AgamPath {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            inner: path.as_ref().to_path_buf(),
        }
    }

    pub fn exists(&self) -> bool {
        self.inner.exists()
    }

    pub fn is_file(&self) -> bool {
        self.inner.is_file()
    }

    pub fn is_dir(&self) -> bool {
        self.inner.is_dir()
    }

    pub fn is_absolute(&self) -> bool {
        self.inner.is_absolute()
    }

    pub fn parent(&self) -> Option<AgamPath> {
        self.inner.parent().map(AgamPath::new)
    }

    pub fn file_name(&self) -> Option<&str> {
        self.inner.file_name().and_then(|s| s.to_str())
    }

    pub fn file_stem(&self) -> Option<&str> {
        self.inner.file_stem().and_then(|s| s.to_str())
    }

    pub fn extension(&self) -> Option<&str> {
        self.inner.extension().and_then(|s| s.to_str())
    }

    pub fn to_str(&self) -> Option<&str> {
        self.inner.to_str()
    }

    pub fn as_path(&self) -> &Path {
        &self.inner
    }

    pub fn join(&self, path: impl AsRef<Path>) -> AgamPath {
        AgamPath::new(self.inner.join(path))
    }
}

impl<P: AsRef<Path>> Div<P> for AgamPath {
    type Output = AgamPath;

    fn div(self, rhs: P) -> Self::Output {
        self.join(rhs)
    }
}

impl<P: AsRef<Path>> Div<P> for &AgamPath {
    type Output = AgamPath;

    fn div(self, rhs: P) -> Self::Output {
        self.join(rhs)
    }
}

impl AsRef<Path> for AgamPath {
    fn as_ref(&self) -> &Path {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!("agam_std_io_{label}_{stamp}"));
        let _ = std::fs::create_dir_all(&path);
        path
    }

    #[test]
    fn write_and_read_text_round_trip() {
        let root = temp_dir("round_trip");
        let file = root.join("nested").join("demo.txt");

        assert!(write_string(&file, "hello\nagam\n").is_ok());
        let res = read_to_string(&file);
        assert!(res.is_ok());
        if let Ok(text) = res {
            assert_eq!(text, "hello\nagam\n");
        }
        assert!(exists(&file));
        assert!(is_file(&file));
        assert!(is_dir(root.join("nested")));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_buffered_reader_writer() {
        let root = temp_dir("buf_rw");
        let file = root.join("buf_test.txt");

        if let Ok(f) = std::fs::File::create(&file) {
            let mut writer = FastBufWriter::new(f);
            assert!(writer.write_line("line 1").is_ok());
            assert!(writer.write_line("line 2").is_ok());
            assert!(writer.flush().is_ok());
        }

        if let Ok(f) = std::fs::File::open(&file) {
            let mut reader = FastBufReader::new(f);
            let l1 = reader.read_line();
            let l2 = reader.read_line();
            let l3 = reader.read_line();

            assert_eq!(l1, Ok(Some("line 1\n".into())));
            assert_eq!(l2, Ok(Some("line 2\n".into())));
            assert_eq!(l3, Ok(None));
        }

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_agam_path_operator_overloads() {
        let p = AgamPath::new("src");
        let sub = p / "main.agam";
        assert_eq!(sub.file_name(), Some("main.agam"));
        assert_eq!(sub.file_stem(), Some("main"));
        assert_eq!(sub.extension(), Some("agam"));
    }
}
