//! # agam_fmt
//!
//! Initial source formatter for the Agam language.
//!
//! This formatter is intentionally conservative today: it preserves comments and
//! source layout, while normalizing line endings, trailing whitespace, leading
//! tab indentation, blank-line runs, and the final newline. That gives the
//! current toolchain a stable formatting command without rewriting user code
//! into a lossy AST printer.

use std::fs;
use std::path::{Path, PathBuf};

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

/// Format a single file in-place unless `check` is set.
pub fn format_path(path: &Path, check: bool) -> Result<bool, String> {
    let source = fs::read_to_string(path)
        .map_err(|e| format!("could not read `{}`: {e}", path.display()))?;
    let formatted = format_source(&source);
    if formatted.changed && !check {
        fs::write(path, formatted.output)
            .map_err(|e| format!("could not write `{}`: {e}", path.display()))?;
    }
    Ok(formatted.changed)
}

/// Format an explicit list of Agam source files.
pub fn format_paths(files: &[PathBuf], check: bool) -> Result<Vec<PathBuf>, String> {
    let mut changed_files = Vec::new();
    for file in files {
        if format_path(file, check)? {
            changed_files.push(file.clone());
        }
    }
    Ok(changed_files)
}

/// Expand user inputs through the shared workspace contract and format the result set.
pub fn format_inputs(inputs: Vec<PathBuf>, check: bool) -> Result<Vec<PathBuf>, String> {
    let files = agam_pkg::expand_agam_inputs(inputs)?;
    format_paths(&files, check)
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

    #[test]
    fn format_paths_writes_changed_files() {
        let dir = std::env::temp_dir().join(format!(
            "agam_fmt_paths_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time should move forward")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let file = dir.join("sample.agam");
        std::fs::write(&file, "fn main() {   \n    return 0; \t\n}").expect("write sample file");

        let changed = format_paths(std::slice::from_ref(&file), false).expect("format paths");

        assert_eq!(changed, vec![file.clone()]);
        assert_eq!(
            std::fs::read_to_string(&file).expect("read formatted"),
            "fn main() {\n    return 0;\n}\n"
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn format_inputs_uses_workspace_expansion() {
        let dir = std::env::temp_dir().join(format!(
            "agam_fmt_workspace_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time should move forward")
                .as_nanos()
        ));
        let entry = dir.join("src").join("main.agam");
        let test_file = dir.join("tests").join("smoke.agam");
        let loose_file = dir.join("notes").join("ignored.agam");
        std::fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        std::fs::create_dir_all(test_file.parent().expect("test parent")).expect("create tests");
        std::fs::create_dir_all(loose_file.parent().expect("loose parent"))
            .expect("create loose dir");
        let manifest = agam_pkg::scaffold_workspace_manifest("fmt-workspace");
        agam_pkg::write_workspace_manifest_to_path(&dir.join("agam.toml"), &manifest)
            .expect("write manifest");
        std::fs::write(&entry, "fn main() {   \n    return 0; \t\n}").expect("write entry");
        std::fs::write(&test_file, "@test\nfn smoke() -> bool:\n    return true\n")
            .expect("write test");
        std::fs::write(&loose_file, "fn ignored() -> i32:\n    return 0\n").expect("write loose");

        let changed = format_inputs(vec![dir.clone()], true).expect("format workspace inputs");

        assert_eq!(changed, vec![entry]);

        let _ = std::fs::remove_dir_all(dir);
    }
}
