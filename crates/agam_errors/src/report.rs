//! Diagnostic emitter — renders diagnostics to the terminal.
//!
//! Produces `rustc`-style error output with colored source snippets,
//! line numbers, and underlined error locations.

use crate::diagnostic::{Diagnostic, DiagnosticLevel};
use crate::span::SourceFile;
use std::collections::HashMap;
use crate::span::SourceId;

/// Collects diagnostics and renders them for the user.
pub struct DiagnosticEmitter {
    /// All emitted diagnostics.
    diagnostics: Vec<Diagnostic>,
    /// Source files for rendering snippets.
    sources: HashMap<SourceId, SourceFile>,
    /// Number of errors emitted.
    error_count: usize,
    /// Number of warnings emitted.
    warning_count: usize,
}

impl DiagnosticEmitter {
    /// Create a new emitter.
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            sources: HashMap::new(),
            error_count: 0,
            warning_count: 0,
        }
    }

    /// Register a source file for diagnostic rendering.
    pub fn add_source(&mut self, file: SourceFile) {
        self.sources.insert(file.id, file);
    }

    /// Emit a diagnostic.
    pub fn emit(&mut self, diagnostic: Diagnostic) {
        match diagnostic.level {
            DiagnosticLevel::Error | DiagnosticLevel::Ice => self.error_count += 1,
            DiagnosticLevel::Warning => self.warning_count += 1,
            DiagnosticLevel::Note => {}
        }
        self.render(&diagnostic);
        self.diagnostics.push(diagnostic);
    }

    /// Render a diagnostic to stderr.
    fn render(&self, diag: &Diagnostic) {
        // Level prefix with color
        let level_str = match diag.level {
            DiagnosticLevel::Error => "\x1b[1;31merror",
            DiagnosticLevel::Warning => "\x1b[1;33mwarning",
            DiagnosticLevel::Note => "\x1b[1;36mnote",
            DiagnosticLevel::Ice => "\x1b[1;31minternal compiler error",
        };

        // Error code
        let code_str = diag
            .code
            .as_ref()
            .map(|c| format!("[{}]", c))
            .unwrap_or_default();

        eprintln!("{}{}\x1b[1;37m: {}\x1b[0m", level_str, code_str, diag.message);

        // Render each label
        for label in &diag.labels {
            if label.span.is_dummy() {
                continue;
            }

            if let Some(source) = self.sources.get(&label.span.source_id) {
                let (line, col) = source.offset_to_line_col(label.span.start as usize);
                let line_text = source.line_text(line);
                let line_num = line + 1;
                let col_num = col + 1;

                // File location
                eprintln!(
                    " \x1b[1;34m-->\x1b[0m {}:{}:{}",
                    source.path, line_num, col_num
                );

                // Line number gutter width
                let gutter_width = format!("{}", line_num).len();

                // Empty gutter line
                eprintln!(" {:>gutter_width$} \x1b[1;34m|\x1b[0m", "");

                // Source line
                eprintln!(
                    " \x1b[1;34m{:>gutter_width$}\x1b[0m \x1b[1;34m|\x1b[0m {}",
                    line_num, line_text
                );

                // Underline
                let span_len = (label.span.end - label.span.start).max(1) as usize;
                let padding = " ".repeat(col);
                let underline_char = if label.is_primary { '^' } else { '-' };
                let color = if label.is_primary { "\x1b[1;31m" } else { "\x1b[1;34m" };
                let underline = std::iter::repeat(underline_char)
                    .take(span_len.min(line_text.len().saturating_sub(col)))
                    .collect::<String>();

                eprintln!(
                    " {:>gutter_width$} \x1b[1;34m|\x1b[0m {}{}{} {}\x1b[0m",
                    "", padding, color, underline, label.message
                );
            }
        }

        // Help text
        if let Some(help) = &diag.help {
            eprintln!(" \x1b[1;36mhelp\x1b[0m: {}", help);
        }

        // Note text
        if let Some(note) = &diag.note {
            eprintln!(" \x1b[1;36mnote\x1b[0m: {}", note);
        }

        eprintln!();
    }

    /// Whether any errors were emitted.
    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }

    /// Get the total error count.
    pub fn error_count(&self) -> usize {
        self.error_count
    }

    /// Get the total warning count.
    pub fn warning_count(&self) -> usize {
        self.warning_count
    }

    /// Print a summary line: "N error(s), M warning(s) emitted".
    pub fn print_summary(&self) {
        if self.error_count > 0 || self.warning_count > 0 {
            let errors = if self.error_count == 1 {
                "1 error".to_string()
            } else {
                format!("{} errors", self.error_count)
            };
            let warnings = if self.warning_count == 1 {
                "1 warning".to_string()
            } else {
                format!("{} warnings", self.warning_count)
            };
            eprintln!(
                "\x1b[1;31m{}\x1b[0m and \x1b[1;33m{}\x1b[0m emitted",
                errors, warnings
            );
        }
    }

    /// Consume the emitter and return all diagnostics.
    pub fn finish(self) -> Vec<Diagnostic> {
        self.diagnostics
    }
}

impl Default for DiagnosticEmitter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{Diagnostic, Label};
    use crate::span::{SourceFile, SourceId, Span};

    #[test]
    fn test_emitter_counts() {
        let mut emitter = DiagnosticEmitter::new();
        emitter.add_source(SourceFile::new(
            SourceId(0),
            "test.agam".into(),
            "let x = 42\n".into(),
        ));

        emitter.emit(Diagnostic::error("E0001", "test error"));
        emitter.emit(Diagnostic::warning("W0001", "test warning"));
        emitter.emit(Diagnostic::note("test note"));

        assert_eq!(emitter.error_count(), 1);
        assert_eq!(emitter.warning_count(), 1);
        assert!(emitter.has_errors());
    }

    #[test]
    fn test_emitter_no_errors() {
        let emitter = DiagnosticEmitter::new();
        assert!(!emitter.has_errors());
        assert_eq!(emitter.error_count(), 0);
    }

    #[test]
    fn test_emit_with_label() {
        let mut emitter = DiagnosticEmitter::new();
        emitter.add_source(SourceFile::new(
            SourceId(0),
            "test.agam".into(),
            "let x: i32 = \"hello\"\n".into(),
        ));

        let diag = Diagnostic::error("E0001", "mismatched types")
            .with_label(Label::primary(
                Span::new(SourceId(0), 14, 21),
                "expected `i32`, found `str`",
            ));

        emitter.emit(diag);
        assert!(emitter.has_errors());
    }
}
