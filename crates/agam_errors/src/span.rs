//! Source location tracking.
//!
//! Every token, AST node, and diagnostic in the compiler carries a [`Span`]
//! that records exactly where in the source code it originated.

use std::fmt;
use std::sync::Arc;

/// Unique identifier for a source file within a compilation session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceId(pub u32);

/// A loaded source file with its contents cached for diagnostics.
#[derive(Debug, Clone)]
pub struct SourceFile {
    /// Unique ID for this file.
    pub id: SourceId,
    /// File path (as provided by the user).
    pub path: String,
    /// Full source text.
    pub source: Arc<str>,
    /// Byte offsets of each line start (for line/column lookups).
    line_starts: Vec<usize>,
}

impl SourceFile {
    /// Create a new source file and compute line start offsets.
    pub fn new(id: SourceId, path: String, source: String) -> Self {
        let line_starts = std::iter::once(0)
            .chain(source.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        Self {
            id,
            path,
            source: Arc::from(source),
            line_starts,
        }
    }

    /// Convert a byte offset to a (line, column) pair (both 0-indexed).
    pub fn offset_to_line_col(&self, offset: usize) -> (usize, usize) {
        let line = self
            .line_starts
            .partition_point(|&start| start <= offset)
            .saturating_sub(1);
        let col = offset - self.line_starts[line];
        (line, col)
    }

    /// Get the text of a specific line (0-indexed).
    pub fn line_text(&self, line: usize) -> &str {
        let start = self.line_starts[line];
        let end = self
            .line_starts
            .get(line + 1)
            .copied()
            .unwrap_or(self.source.len());
        &self.source[start..end]
            .trim_end_matches('\n')
            .trim_end_matches('\r')
    }

    /// Total number of lines.
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }
}

/// A span representing a contiguous range of bytes in a source file.
///
/// Spans are the fundamental building block for error reporting — they
/// tell the user exactly where an error occurred.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    /// Which source file this span belongs to.
    pub source_id: SourceId,
    /// Byte offset of the start of the span (inclusive).
    pub start: u32,
    /// Byte offset of the end of the span (exclusive).
    pub end: u32,
}

impl Span {
    /// Create a new span.
    pub fn new(source_id: SourceId, start: u32, end: u32) -> Self {
        debug_assert!(start <= end, "Span start must be <= end");
        Self {
            source_id,
            start,
            end,
        }
    }

    /// Create a zero-length span at a specific offset (for pointing at a position).
    pub fn point(source_id: SourceId, offset: u32) -> Self {
        Self {
            source_id,
            start: offset,
            end: offset,
        }
    }

    /// Merge two spans into one that covers both.
    pub fn merge(self, other: Span) -> Span {
        debug_assert_eq!(
            self.source_id, other.source_id,
            "Cannot merge spans from different files"
        );
        Span {
            source_id: self.source_id,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// Length of this span in bytes.
    pub fn len(&self) -> u32 {
        self.end - self.start
    }

    /// Whether the span is empty.
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// A dummy span for compiler-generated nodes with no source location.
    pub fn dummy() -> Self {
        Self {
            source_id: SourceId(u32::MAX),
            start: 0,
            end: 0,
        }
    }

    /// Check if this is a dummy span.
    pub fn is_dummy(&self) -> bool {
        self.source_id.0 == u32::MAX
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Span({}..{} in {:?})",
            self.start, self.end, self.source_id
        )
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_file() -> SourceFile {
        SourceFile::new(
            SourceId(0),
            "test.agam".to_string(),
            "fn main():\n    print(\"hello\")\n    return 0\n".to_string(),
        )
    }

    #[test]
    fn test_line_col_first_line() {
        let file = sample_file();
        let (line, col) = file.offset_to_line_col(3); // 'main'
        assert_eq!(line, 0);
        assert_eq!(col, 3);
    }

    #[test]
    fn test_line_col_second_line() {
        let file = sample_file();
        // "fn main():\n" = 11 chars, so offset 11 = start of line 2
        let (line, col) = file.offset_to_line_col(11);
        assert_eq!(line, 1);
        assert_eq!(col, 0);
    }

    #[test]
    fn test_line_text() {
        let file = sample_file();
        assert_eq!(file.line_text(0), "fn main():");
        assert_eq!(file.line_text(1), "    print(\"hello\")");
        assert_eq!(file.line_text(2), "    return 0");
    }

    #[test]
    fn test_span_merge() {
        let s1 = Span::new(SourceId(0), 0, 5);
        let s2 = Span::new(SourceId(0), 10, 20);
        let merged = s1.merge(s2);
        assert_eq!(merged.start, 0);
        assert_eq!(merged.end, 20);
    }

    #[test]
    fn test_span_point() {
        let s = Span::point(SourceId(0), 42);
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn test_dummy_span() {
        let s = Span::dummy();
        assert!(s.is_dummy());
    }
}
