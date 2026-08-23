//! Source location tracking and span validation.
//!
//! Every token, AST node, and diagnostic in the compiler carries a [`Span`]
//! that records where in the source code it originated.
//!
//! [`ValidatedSpan`] guarantees that byte ranges are strictly within bounds
//! and aligned to UTF-8 character boundaries, preventing any runtime panics.

use std::fmt;
use std::sync::Arc;

/// Unique identifier for a source file within a compilation session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourceId(pub u32);

/// Align an arbitrary byte index down to the nearest preceding UTF-8 character boundary.
#[inline]
pub fn floor_char_boundary(s: &str, mut index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    while index > 0 && !s.is_char_boundary(index) {
        index -= 1;
    }
    index
}

/// Align an arbitrary byte index up to the nearest succeeding UTF-8 character boundary.
#[inline]
pub fn ceil_char_boundary(s: &str, mut index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    while index < s.len() && !s.is_char_boundary(index) {
        index += 1;
    }
    index
}

/// A validated, bounds-checked and UTF-8-aligned span.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValidatedSpan {
    pub source_id: u32,
    pub start_byte: u32,
    pub end_byte: u32,
}

impl ValidatedSpan {
    /// Create a validated span clamped to a maximum byte length without char boundary alignment.
    pub fn new_clamped(source_id: u32, start: u32, end: u32, src_len: usize) -> Self {
        let max_len = src_len as u32;
        let clamped_start = start.min(max_len);
        let clamped_end = end.min(max_len).max(clamped_start);
        Self {
            source_id,
            start_byte: clamped_start,
            end_byte: clamped_end,
        }
    }

    /// Create a validated span strictly aligned to UTF-8 character boundaries and clamped to string length.
    pub fn from_str_clamped(source_id: u32, start: u32, end: u32, text: &str) -> Self {
        let text_len = text.len();
        if text_len == 0 {
            return Self {
                source_id,
                start_byte: 0,
                end_byte: 0,
            };
        }

        let clamped_start = (start as usize).min(text_len);
        let clamped_end = (end as usize).min(text_len);

        let aligned_start = floor_char_boundary(text, clamped_start);
        let aligned_end = ceil_char_boundary(text, clamped_end).max(aligned_start);

        Self {
            source_id,
            start_byte: aligned_start as u32,
            end_byte: aligned_end as u32,
        }
    }

    /// Convert to regular Span.
    pub fn as_span(&self) -> Span {
        Span {
            source_id: SourceId(self.source_id),
            start: self.start_byte,
            end: self.end_byte,
        }
    }

    /// Length in bytes.
    #[inline]
    pub fn len(&self) -> u32 {
        self.end_byte.saturating_sub(self.start_byte)
    }

    /// Whether span is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.start_byte >= self.end_byte
    }

    /// Extract the sliced substring from the given source text safely.
    pub fn slice<'a>(&self, text: &'a str) -> &'a str {
        let start = (self.start_byte as usize).min(text.len());
        let end = (self.end_byte as usize).min(text.len()).max(start);
        let safe_start = floor_char_boundary(text, start);
        let safe_end = ceil_char_boundary(text, end).max(safe_start);
        &text[safe_start..safe_end]
    }
}

impl fmt::Debug for ValidatedSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ValidatedSpan({}..{} in file #{})",
            self.start_byte, self.end_byte, self.source_id
        )
    }
}

impl fmt::Display for ValidatedSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start_byte, self.end_byte)
    }
}

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
    ///
    /// Clamps safely even if the offset exceeds source length.
    pub fn offset_to_line_col(&self, offset: usize) -> (usize, usize) {
        if self.line_starts.is_empty() {
            return (0, 0);
        }
        let clamped_offset = offset.min(self.source.len());
        let line = self
            .line_starts
            .partition_point(|&start| start <= clamped_offset)
            .saturating_sub(1);
        let line_start = self.line_starts.get(line).copied().unwrap_or(0);

        // Compute UTF-8 character column for proper visual alignment
        let line_str = self.safe_slice(line_start, clamped_offset);
        let char_col = line_str.chars().count();
        (line, char_col)
    }

    /// Safe byte-level slice of source text aligned to UTF-8 character boundaries.
    pub fn safe_slice(&self, start: usize, end: usize) -> &str {
        let text = self.source.as_ref();
        let len = text.len();
        if len == 0 {
            return "";
        }
        let clamped_start = start.min(len);
        let clamped_end = end.min(len).max(clamped_start);
        let safe_start = floor_char_boundary(text, clamped_start);
        let safe_end = ceil_char_boundary(text, clamped_end).max(safe_start);
        &text[safe_start..safe_end]
    }

    /// Get the text of a specific line (0-indexed) with safe UTF-8 slicing.
    pub fn line_text(&self, line: usize) -> &str {
        if line >= self.line_starts.len() {
            return "";
        }
        let start = self.line_starts[line];
        let end = self
            .line_starts
            .get(line + 1)
            .copied()
            .unwrap_or(self.source.len());
        self.safe_slice(start, end)
            .trim_end_matches('\n')
            .trim_end_matches('\r')
    }

    /// Total number of lines.
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Validate and clamp an unbounded Span against this source file.
    pub fn validate_span(&self, span: Span) -> ValidatedSpan {
        ValidatedSpan::from_str_clamped(span.source_id.0, span.start, span.end, &self.source)
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
        let ordered_start = start.min(end);
        let ordered_end = start.max(end);
        Self {
            source_id,
            start: ordered_start,
            end: ordered_end,
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
        Span {
            source_id: self.source_id,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// Length of this span in bytes.
    pub fn len(&self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    /// Whether the span is empty.
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
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

    /// Validate and clamp this span against a source text.
    pub fn validate(&self, text: &str) -> ValidatedSpan {
        ValidatedSpan::from_str_clamped(self.source_id.0, self.start, self.end, text)
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

    #[test]
    fn test_validated_span_clamping_and_bounds() {
        let val = ValidatedSpan::new_clamped(1, 100, 200, 50);
        assert_eq!(val.start_byte, 50);
        assert_eq!(val.end_byte, 50);
        assert!(val.is_empty());

        let val2 = ValidatedSpan::new_clamped(1, 10, 40, 100);
        assert_eq!(val2.start_byte, 10);
        assert_eq!(val2.end_byte, 40);
        assert_eq!(val2.len(), 30);
    }

    #[test]
    fn test_validated_span_utf8_char_boundary_alignment() {
        // Multi-byte Unicode: 'न' is 3 bytes (offsets 0..3), 'म' is 3 bytes (offsets 3..6)
        let text = "नमस्ते";
        assert_eq!(text.len(), 18); // 6 Devanagari chars * 3 bytes each

        // Mid-character slicing offset 1..4 should snap to 0..6
        let val = ValidatedSpan::from_str_clamped(0, 1, 4, text);
        assert_eq!(val.start_byte, 0);
        assert_eq!(val.end_byte, 6);
        assert_eq!(val.slice(text), "नम");

        // Tamil multi-byte text: "அகம்"
        let tamil = "அகம்";
        let val_tamil = ValidatedSpan::from_str_clamped(0, 2, 7, tamil);
        assert!(val_tamil.start_byte <= val_tamil.end_byte);
        let sliced = val_tamil.slice(tamil);
        assert!(!sliced.is_empty());
    }

    #[test]
    fn test_source_file_safe_slicing_out_of_bounds() {
        let file = SourceFile::new(SourceId(42), "safe.agam".into(), "let x = 1;\n".into());
        // Requesting line 999 or offset 999 must never panic
        assert_eq!(file.line_text(999), "");
        let (l, c) = file.offset_to_line_col(999);
        assert_eq!(l, 1);
        assert_eq!(c, 0);

        assert_eq!(file.safe_slice(100, 200), "");
    }
}
