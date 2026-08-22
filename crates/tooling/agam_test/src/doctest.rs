//! Documentation Test Extraction & Execution Engine.
//!
//! Extracts code blocks from doc comments (`/// ```agam ... ``` `)
//! and evaluates them as executable doctests.

/// Extracted documentation test case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocTestCase {
    pub file: String,
    pub line: usize,
    pub source: String,
    pub should_panic: bool,
    pub compile_fail: bool,
}

/// Extractor for doc comments in source files.
pub struct DocTestExtractor;

impl DocTestExtractor {
    /// Extract all doctest blocks from an Agam source string.
    pub fn extract(file_name: &str, content: &str) -> Vec<DocTestCase> {
        let mut tests = Vec::new();
        let mut in_code_block = false;
        let mut current_block = Vec::new();
        let mut start_line = 0;
        let mut should_panic = false;
        let mut compile_fail = false;

        for (idx, line) in content.lines().enumerate() {
            let line_num = idx + 1;
            let trimmed = line.trim();

            if trimmed.starts_with("/// ```") || trimmed.starts_with("//! ```") {
                if !in_code_block {
                    in_code_block = true;
                    start_line = line_num;
                    current_block.clear();

                    let header = trimmed
                        .trim_start_matches("///")
                        .trim_start_matches("//!")
                        .trim()
                        .trim_start_matches("```");

                    should_panic = header.contains("should_panic");
                    compile_fail = header.contains("compile_fail");
                } else {
                    in_code_block = false;
                    tests.push(DocTestCase {
                        file: file_name.to_string(),
                        line: start_line,
                        source: current_block.join("\n"),
                        should_panic,
                        compile_fail,
                    });
                }
            } else if in_code_block {
                let code_line = if let Some(stripped) = trimmed.strip_prefix("/// ") {
                    stripped
                } else if let Some(stripped) = trimmed.strip_prefix("//! ") {
                    stripped
                } else if trimmed == "///" || trimmed == "//!" {
                    ""
                } else {
                    trimmed
                };
                current_block.push(code_line.to_string());
            }
        }

        tests
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_doctests_from_doc_comments() {
        let doc_src = r#"
/// Adds two numbers.
///
/// ```agam
/// let res = add(2, 3);
/// assert(res == 5);
/// ```
fn add(a: i64, b: i64) -> i64 {
    a + b
}

/// Must panic on overflow.
/// ```agam,should_panic
/// panic("overflow");
/// ```
fn trigger_panic() {}
"#;

        let tests = DocTestExtractor::extract("math.agam", doc_src);
        assert_eq!(tests.len(), 2);
        assert_eq!(tests[0].file, "math.agam");
        assert!(!tests[0].should_panic);
        assert!(tests[0].source.contains("let res = add(2, 3);"));

        assert!(tests[1].should_panic);
        assert!(tests[1].source.contains("panic(\"overflow\");"));
    }
}
