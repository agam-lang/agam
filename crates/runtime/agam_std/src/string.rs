//! First-party string formatting, builder, and UTF-8 safe scanning utilities.
//!
//! Provides capacity-preallocated string builders and non-panicking UTF-8 character
//! boundary slicing algorithms per `note.md` and `ADOPTED_DEPENDENCIES.md`.

#![deny(clippy::unwrap_used)]

use std::fmt;

/// High-throughput string accumulator with capacity preallocation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StringBuilder {
    buffer: String,
}

impl StringBuilder {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: String::with_capacity(capacity),
        }
    }

    pub fn append(&mut self, text: impl AsRef<str>) -> &mut Self {
        self.buffer.push_str(text.as_ref());
        self
    }

    pub fn append_line(&mut self, text: impl AsRef<str>) -> &mut Self {
        self.buffer.push_str(text.as_ref());
        self.buffer.push('\n');
        self
    }

    pub fn append_char(&mut self, ch: char) -> &mut Self {
        self.buffer.push(ch);
        self
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn as_str(&self) -> &str {
        &self.buffer
    }

    pub fn finish(self) -> String {
        self.buffer
    }
}

impl fmt::Display for StringBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.buffer)
    }
}

/// UTF-8 safe character boundary scanner and indexer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Utf8Scanner<'a> {
    text: &'a str,
}

impl<'a> Utf8Scanner<'a> {
    pub fn new(text: &'a str) -> Self {
        Self { text }
    }

    /// Check if byte index falls cleanly on a UTF-8 character boundary.
    pub fn is_char_boundary(&self, byte_offset: usize) -> bool {
        self.text.is_char_boundary(byte_offset)
    }

    /// Total number of Unicode scalar values (characters) in the string.
    pub fn char_count(&self) -> usize {
        self.text.chars().count()
    }

    /// Return the character at the given 0-indexed character index.
    pub fn char_at(&self, char_index: usize) -> Option<char> {
        self.text.chars().nth(char_index)
    }

    /// Convert a 0-indexed character index into its corresponding byte offset.
    pub fn byte_offset_of_char(&self, char_index: usize) -> Option<usize> {
        let mut count = 0;
        for (offset, _) in self.text.char_indices() {
            if count == char_index {
                return Some(offset);
            }
            count += 1;
        }
        if count == char_index {
            Some(self.text.len())
        } else {
            None
        }
    }

    /// Safe substring slicing by character index without UTF-8 boundary panics.
    pub fn substring_chars(&self, start_char: usize, end_char: usize) -> Option<&'a str> {
        if start_char > end_char {
            return None;
        }
        let start_byte = self.byte_offset_of_char(start_char)?;
        let end_byte = self.byte_offset_of_char(end_char)?;
        if start_byte <= end_byte && end_byte <= self.text.len() {
            Some(&self.text[start_byte..end_byte])
        } else {
            None
        }
    }

    /// Safe substring slicing by byte offset without UTF-8 boundary panics.
    pub fn substring_bytes_safe(&self, start_byte: usize, end_byte: usize) -> Option<&'a str> {
        if start_byte <= end_byte
            && end_byte <= self.text.len()
            && self.is_char_boundary(start_byte)
            && self.is_char_boundary(end_byte)
        {
            Some(&self.text[start_byte..end_byte])
        } else {
            None
        }
    }
}

/// Case-folding equality check (case-insensitive ASCII string comparison).
pub fn case_fold_eq(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_builder() {
        let mut sb = StringBuilder::with_capacity(32);
        sb.append("Hello").append(", ").append_line("World!");
        sb.append_char('4').append_char('2');

        assert_eq!(sb.as_str(), "Hello, World!\n42");
        assert_eq!(sb.len(), 16);
        assert_eq!(sb.finish(), "Hello, World!\n42");
    }

    #[test]
    fn test_utf8_scanner_multibyte_safety() {
        // "agam 🚀 தமிழ்" contains 1-byte, 4-byte, and 3-byte UTF-8 sequences
        let s = "agam 🚀 தமிழ்";
        let scanner = Utf8Scanner::new(s);

        assert_eq!(scanner.char_at(0), Some('a'));
        assert_eq!(scanner.char_at(5), Some('🚀'));

        // Substring by character positions
        assert_eq!(scanner.substring_chars(0, 4), Some("agam"));
        assert_eq!(scanner.substring_chars(5, 6), Some("🚀"));

        // Invalid byte offsets return None instead of panicking
        assert_eq!(scanner.substring_bytes_safe(0, 6), None); // Byte 6 is mid-rocket
        assert_eq!(scanner.substring_bytes_safe(0, 9), Some("agam 🚀"));
    }

    #[test]
    fn test_case_fold_eq() {
        assert!(case_fold_eq("AgamLang", "agamlang"));
        assert!(case_fold_eq("HTTP_HEADER", "http_header"));
        assert!(!case_fold_eq("hello", "world"));
    }
}
