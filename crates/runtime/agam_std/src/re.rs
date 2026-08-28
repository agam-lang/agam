//! First-party regular expression matching and transformation powered by `regex`.
//!
//! Enforces the Facade-Completeness & Zero-Identity-Leak Invariant per `ADOPTED_DEPENDENCIES.md`
//! and `note.md`: guaranteed $O(n)$ search time with native Nyāya error diagnostics.

#![deny(clippy::unwrap_used)]

use std::fmt;

use regex::Regex as InternalRegex;

/// Structured regex compilation or matching diagnostic formatted in the Agam Nyāya voice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegexError {
    pub cause: String,
    pub context: String,
    pub remedy: String,
}

impl RegexError {
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

impl fmt::Display for RegexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Regex Diagnostic: {}\n  Context: {}\n  Remedy:  {}",
            self.cause, self.context, self.remedy
        )
    }
}

impl std::error::Error for RegexError {}

/// Captured regex match metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegexMatch {
    pub matched: String,
    pub start: usize,
    pub end: usize,
}

/// Compiled regular expression pattern.
#[derive(Clone, Debug)]
pub struct Regex {
    inner: InternalRegex,
}

impl Regex {
    /// Compile a regex pattern string into a verified DFA/NFA engine.
    pub fn compile(pattern: &str) -> Result<Self, RegexError> {
        InternalRegex::new(pattern)
            .map(|inner| Self { inner })
            .map_err(|e| RegexError {
                cause: format!("Invalid regular expression syntax: {}", e),
                context: format!("Pattern '{}' failed regular expression validation", pattern),
                remedy: "Verify escape sequences, unclosed delimiters, and character classes"
                    .to_string(),
            })
    }

    /// Check if the regular expression matches any portion of `text`.
    pub fn is_match(&self, text: &str) -> bool {
        self.inner.is_match(text)
    }

    /// Return the first matching substring and its byte span.
    pub fn find(&self, text: &str) -> Option<RegexMatch> {
        self.inner.find(text).map(|m| RegexMatch {
            matched: m.as_str().to_string(),
            start: m.start(),
            end: m.end(),
        })
    }

    /// Return all non-overlapping matching substrings.
    pub fn find_all(&self, text: &str) -> Vec<String> {
        self.inner
            .find_iter(text)
            .map(|m| m.as_str().to_string())
            .collect()
    }

    /// Return all non-overlapping matches with byte spans.
    pub fn find_iter(&self, text: &str) -> Vec<RegexMatch> {
        self.inner
            .find_iter(text)
            .map(|m| RegexMatch {
                matched: m.as_str().to_string(),
                start: m.start(),
                end: m.end(),
            })
            .collect()
    }

    /// Replace all matching occurrences with `replacement`.
    pub fn replace(&self, text: &str, replacement: &str) -> String {
        self.inner.replace_all(text, replacement).to_string()
    }

    /// Split `text` by regex matches into a vector of substrings.
    pub fn split(&self, text: &str) -> Vec<String> {
        self.inner.split(text).map(|s| s.to_string()).collect()
    }
}

/// Dynamic top-level function: search for pattern in text.
pub fn search(pattern: &str, text: &str) -> Result<Option<RegexMatch>, RegexError> {
    let re = Regex::compile(pattern)?;
    Ok(re.find(text))
}

/// Dynamic top-level function: check if pattern matches text.
pub fn is_match(pattern: &str, text: &str) -> Result<bool, RegexError> {
    let re = Regex::compile(pattern)?;
    Ok(re.is_match(text))
}

/// Dynamic top-level function: find all matching substrings.
pub fn find_all(pattern: &str, text: &str) -> Result<Vec<String>, RegexError> {
    let re = Regex::compile(pattern)?;
    Ok(re.find_all(text))
}

/// Dynamic top-level function: replace all matches with replacement.
pub fn replace(pattern: &str, text: &str, replacement: &str) -> Result<String, RegexError> {
    let re = Regex::compile(pattern)?;
    Ok(re.replace(text, replacement))
}

/// Dynamic top-level function: split text by pattern.
pub fn split(pattern: &str, text: &str) -> Result<Vec<String>, RegexError> {
    let re = Regex::compile(pattern)?;
    Ok(re.split(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_compile_and_find() {
        let re_res = Regex::compile(r"\d+");
        assert!(re_res.is_ok());
        if let Ok(re) = re_res {
            let m = re.find("Agam version 42 released");
            assert!(m.is_some());
            if let Some(match_meta) = m {
                assert_eq!(match_meta.matched, "42");
                assert_eq!(match_meta.start, 13);
                assert_eq!(match_meta.end, 15);
            }
        }
    }

    #[test]
    fn test_regex_find_all_and_replace() {
        let re_res = Regex::compile(r"[a-z]+");
        assert!(re_res.is_ok());
        if let Ok(re) = re_res {
            let matches = re.find_all("123 hello 456 world");
            assert_eq!(matches, vec!["hello", "world"]);

            let replaced = re.replace("123 hello 456 world", "token");
            assert_eq!(replaced, "123 token 456 token");
        }
    }

    #[test]
    fn test_regex_invalid_pattern_returns_nyaya_error() {
        let bad = Regex::compile(r"[a-z(");
        assert!(bad.is_err());
        if let Err(e) = bad {
            assert!(e.to_string().contains("Regex Diagnostic"));
        }
    }
}
