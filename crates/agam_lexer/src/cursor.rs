//! Character-level cursor for reading source text.
//!
//! The cursor provides a stream of characters with position tracking,
//! peek-ahead, and UTF-8-aware iteration.

#![allow(dead_code)]

/// A cursor over a source string with position tracking.
pub struct Cursor<'src> {
    /// The full source text.
    source: &'src str,
    /// Remaining unconsumed portion of the source.
    rest: &'src str,
    /// Current byte offset into the source.
    pos: usize,
}

impl<'src> Cursor<'src> {
    /// Create a new cursor at the start of the source.
    pub fn new(source: &'src str) -> Self {
        Self {
            source,
            rest: source,
            pos: 0,
        }
    }

    /// Current byte position.
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// Whether we've consumed all characters.
    pub fn is_eof(&self) -> bool {
        self.rest.is_empty()
    }

    /// Peek at the next character without consuming it.
    pub fn peek(&self) -> Option<char> {
        self.rest.chars().next()
    }

    /// Peek at the character after the next one.
    pub fn peek_second(&self) -> Option<char> {
        let mut chars = self.rest.chars();
        chars.next();
        chars.next()
    }

    /// Peek at the next n characters as a string slice.
    pub fn peek_str(&self, n: usize) -> &'src str {
        let end = self.rest.char_indices()
            .nth(n)
            .map(|(i, _)| i)
            .unwrap_or(self.rest.len());
        &self.rest[..end]
    }

    /// Peek at the previous character.
    pub fn peek_prev(&self) -> Option<char> {
        if self.pos > 0 {
            self.source[..self.pos].chars().next_back()
        } else {
            None
        }
    }

    /// Reset the cursor to a specific byte position.
    pub fn reset_pos(&mut self, pos: usize) {
        if pos <= self.source.len() {
            self.pos = pos;
            self.rest = &self.source[pos..];
        }
    }

    /// Consume and return the next character.
    pub fn advance(&mut self) -> Option<char> {
        let c = self.rest.chars().next()?;
        let len = c.len_utf8();
        self.rest = &self.rest[len..];
        self.pos += len;
        Some(c)
    }

    /// Consume characters while the predicate is true.
    pub fn eat_while(&mut self, mut predicate: impl FnMut(char) -> bool) {
        while let Some(c) = self.peek() {
            if predicate(c) {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Consume characters while they match a specific character.
    pub fn eat_char(&mut self, expected: char) -> usize {
        let mut count = 0;
        while self.peek() == Some(expected) {
            self.advance();
            count += 1;
        }
        count
    }

    /// Check if the remaining source starts with a given string.
    pub fn starts_with(&self, s: &str) -> bool {
        self.rest.starts_with(s)
    }

    /// Get a slice of the source from the given start position to the current position.
    pub fn slice_from(&self, start: usize) -> &'src str {
        &self.source[start..self.pos]
    }

    /// Get the remaining unconsumed source.
    pub fn remaining(&self) -> &'src str {
        self.rest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_advance() {
        let mut cursor = Cursor::new("abc");
        assert_eq!(cursor.advance(), Some('a'));
        assert_eq!(cursor.advance(), Some('b'));
        assert_eq!(cursor.advance(), Some('c'));
        assert_eq!(cursor.advance(), None);
        assert!(cursor.is_eof());
    }

    #[test]
    fn test_peek() {
        let cursor = Cursor::new("hello");
        assert_eq!(cursor.peek(), Some('h'));
        assert_eq!(cursor.peek_second(), Some('e'));
    }

    #[test]
    fn test_eat_while() {
        let mut cursor = Cursor::new("1234abc");
        cursor.eat_while(|c| c.is_ascii_digit());
        assert_eq!(cursor.pos(), 4);
        assert_eq!(cursor.peek(), Some('a'));
    }

    #[test]
    fn test_utf8() {
        let mut cursor = Cursor::new("héllo");
        assert_eq!(cursor.advance(), Some('h'));
        assert_eq!(cursor.advance(), Some('é'));
        assert_eq!(cursor.pos(), 3); // 'é' is 2 bytes in UTF-8
    }

    #[test]
    fn test_slice_from() {
        let mut cursor = Cursor::new("fn main()");
        let start = cursor.pos();
        cursor.advance(); // f
        cursor.advance(); // n
        assert_eq!(cursor.slice_from(start), "fn");
    }
}
