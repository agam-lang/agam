//! First-party RFC 4180 CSV parsing and serialization powered by `csv`.
//!
//! Enforces the Facade-Completeness & Zero-Identity-Leak Invariant per `ADOPTED_DEPENDENCIES.md`
//! and `note.md`: robust quoted field handling with native Nyāya error diagnostics.

#![deny(clippy::unwrap_used)]

use std::fmt;
use std::fs::File;
use std::path::Path;

use csv::{ReaderBuilder, WriterBuilder};

/// Structured CSV I/O or parsing diagnostic formatted in the Agam Nyāya voice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvError {
    pub cause: String,
    pub context: String,
    pub remedy: String,
}

impl CsvError {
    pub fn new(
        cause: impl fmt::Display,
        context: impl fmt::Display,
        remedy: impl fmt::Display,
    ) -> Self {
        Self {
            cause: cause.to_string(),
            context: context.to_string(),
            remedy: remedy.to_string(),
        }
    }
}

impl fmt::Display for CsvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CSV Diagnostic: {}\n  Context: {}\n  Remedy:  {}",
            self.cause, self.context, self.remedy
        )
    }
}

impl std::error::Error for CsvError {}

/// Parse a CSV formatted in-memory string into a 2D vector of cell strings.
pub fn parse_csv_string(csv_text: &str) -> Result<Vec<Vec<String>>, CsvError> {
    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .from_reader(csv_text.as_bytes());

    let mut rows = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|e| {
            CsvError::new(
                format!("Failed to parse CSV record: {}", e),
                "Encountered malformed CSV content during record streaming",
                "Verify quote escaping and delimiter consistency",
            )
        })?;
        rows.push(record.iter().map(|s| s.to_string()).collect());
    }
    Ok(rows)
}

/// Read a CSV file into records without header separation.
pub fn read_records(path: impl AsRef<Path>) -> Result<Vec<Vec<String>>, CsvError> {
    let p = path.as_ref();
    let file = File::open(p).map_err(|e| {
        CsvError::new(
            format!("Cannot open CSV file: {}", e),
            format!("Path: '{}'", p.display()),
            "Verify that the file exists and has read permissions",
        )
    })?;

    let mut reader = ReaderBuilder::new().has_headers(false).from_reader(file);
    let mut rows = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|e| {
            CsvError::new(
                format!("Failed to read CSV record: {}", e),
                format!("File: '{}'", p.display()),
                "Verify CSV structure and character encoding",
            )
        })?;
        rows.push(record.iter().map(|s| s.to_string()).collect());
    }
    Ok(rows)
}

/// Read a CSV file separating the header row from subsequent data records.
pub fn read_records_with_headers(
    path: impl AsRef<Path>,
) -> Result<(Vec<String>, Vec<Vec<String>>), CsvError> {
    let p = path.as_ref();
    let file = File::open(p).map_err(|e| {
        CsvError::new(
            format!("Cannot open CSV file: {}", e),
            format!("Path: '{}'", p.display()),
            "Verify that the file exists and has read permissions",
        )
    })?;

    let mut reader = ReaderBuilder::new().has_headers(true).from_reader(file);
    let headers = reader
        .headers()
        .map_err(|e| {
            CsvError::new(
                format!("Failed to read CSV headers: {}", e),
                format!("File: '{}'", p.display()),
                "Ensure the first line contains valid column names",
            )
        })?
        .iter()
        .map(|s| s.to_string())
        .collect();

    let mut rows = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|e| {
            CsvError::new(
                format!("Failed to read CSV data row: {}", e),
                format!("File: '{}'", p.display()),
                "Verify row column alignment with headers",
            )
        })?;
        rows.push(record.iter().map(|s| s.to_string()).collect());
    }
    Ok((headers, rows))
}

/// Write headers and row records into a CSV file.
pub fn write_records(
    path: impl AsRef<Path>,
    headers: Option<&[&str]>,
    rows: &[Vec<String>],
) -> Result<(), CsvError> {
    let p = path.as_ref();
    let file = File::create(p).map_err(|e| {
        CsvError::new(
            format!("Cannot create CSV file: {}", e),
            format!("Path: '{}'", p.display()),
            "Verify write permissions for the destination directory",
        )
    })?;

    let mut writer = WriterBuilder::new().from_writer(file);
    if let Some(hdrs) = headers {
        writer.write_record(hdrs).map_err(|e| {
            CsvError::new(
                format!("Failed writing CSV headers: {}", e),
                format!("File: '{}'", p.display()),
                "Check disk space and stream health",
            )
        })?;
    }

    for row in rows {
        writer.write_record(row).map_err(|e| {
            CsvError::new(
                format!("Failed writing CSV row: {}", e),
                format!("File: '{}'", p.display()),
                "Check disk space and stream health",
            )
        })?;
    }

    writer.flush().map_err(|e| {
        CsvError::new(
            format!("Failed flushing CSV file: {}", e),
            format!("File: '{}'", p.display()),
            "Ensure buffer flush completes cleanly",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_csv_string_with_quotes() {
        let text = "name,score,role\n\"Vikash, Reddy\",99,\"Compiler Lead\"\nAgam,100,Language";
        let parsed = parse_csv_string(text);
        assert!(parsed.is_ok());
        if let Ok(rows) = parsed {
            assert_eq!(rows.len(), 3);
            assert_eq!(rows[1][0], "Vikash, Reddy");
            assert_eq!(rows[1][1], "99");
            assert_eq!(rows[1][2], "Compiler Lead");
            assert_eq!(rows[2][0], "Agam");
        }
    }

    #[test]
    fn test_read_nonexistent_csv_returns_nyaya_error() {
        let bad = read_records("nonexistent_file_path_12345.csv");
        assert!(bad.is_err());
        if let Err(e) = bad {
            assert!(e.to_string().contains("CSV Diagnostic"));
        }
    }
}
