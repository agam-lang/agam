//! # agam_fmt
//!
//! Initial source formatter for the Agam language.
//!
//! This formatter is intentionally conservative today: it preserves comments and
//! source layout, while normalizing line endings, trailing whitespace, leading
//! tab indentation, blank-line runs, and the final newline. That gives the
//! current toolchain a stable formatting command without rewriting user code
//! into a lossy AST printer.

/// Formatting options for the current whitespace-stable formatter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatOptions {
    /// Maximum allowed run of consecutive blank lines.
    pub max_consecutive_blank_lines: usize,
    /// Number of spaces to expand a leading tab into.
    pub indent_width: usize,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            max_consecutive_blank_lines: 1,
            indent_width: 4,
        }
    }
}

/// Result of formatting a source string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatOutcome {
    pub output: String,
    pub changed: bool,
}

/// Format source with default options.
pub fn format_source(source: &str) -> FormatOutcome {
    format_source_with_options(source, FormatOptions::default())
}

/// Format source with explicit options.
pub fn format_source_with_options(source: &str, options: FormatOptions) -> FormatOutcome {
    let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
    let mut output_lines = Vec::new();
    let mut blank_run = 0usize;

    for line in normalized.split('\n') {
        let trimmed = trim_trailing_whitespace(line);
        let normalized_indent = normalize_leading_tabs(trimmed, options.indent_width);

        if normalized_indent.is_empty() {
            blank_run += 1;
            if !output_lines.is_empty() && blank_run <= options.max_consecutive_blank_lines {
                output_lines.push(String::new());
            }
            continue;
        }

        blank_run = 0;
        output_lines.push(normalized_indent);
    }

    while matches!(output_lines.last(), Some(line) if line.is_empty()) {
        output_lines.pop();
    }

    let mut output = output_lines.join("\n");
    output.push('\n');

    FormatOutcome {
        changed: output != source,
        output,
    }
}

fn trim_trailing_whitespace(line: &str) -> &str {
    line.trim_end_matches([' ', '\t'])
}

fn normalize_leading_tabs(line: &str, indent_width: usize) -> String {
    let mut end = 0usize;
    let mut saw_tab = false;

    for (idx, ch) in line.char_indices() {
        if ch == ' ' || ch == '\t' {
            if ch == '\t' {
                saw_tab = true;
            }
            end = idx + ch.len_utf8();
        } else {
            break;
        }
    }

    if end == 0 || !saw_tab {
        return line.to_string();
    }

    let mut out = String::with_capacity(line.len() + indent_width);
    for ch in line[..end].chars() {
        match ch {
            '\t' => out.push_str(&" ".repeat(indent_width)),
            ' ' => out.push(' '),
            _ => {}
        }
    }
    out.push_str(&line[end..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_trailing_whitespace_and_ensures_final_newline() {
        let formatted = format_source("fn main() {   \n    return 0; \t\n}");
        assert!(formatted.changed);
        assert_eq!(formatted.output, "fn main() {\n    return 0;\n}\n");
    }

    #[test]
    fn normalizes_crlf_and_collapses_blank_runs() {
        let formatted =
            format_source("@lang.advance\r\n\r\n\r\nfn main() {\r\n    return 0;\r\n}\r\n");
        assert_eq!(
            formatted.output,
            "@lang.advance\n\nfn main() {\n    return 0;\n}\n"
        );
    }

    #[test]
    fn expands_leading_tabs_without_touching_inline_tabs() {
        let formatted = format_source("@lang.base\n\tlet x = 1\n\tprint(\"a\\tb\")\n");
        assert_eq!(
            formatted.output,
            "@lang.base\n    let x = 1\n    print(\"a\\tb\")\n"
        );
    }

    #[test]
    fn preserves_already_formatted_source() {
        let source = "@lang.advance\nfn main() {\n    return 0;\n}\n";
        let formatted = format_source(source);
        assert!(!formatted.changed);
        assert_eq!(formatted.output, source);
    }
}
