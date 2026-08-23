//! Diagnostic emitter — renders diagnostics to the terminal.
//!
//! Produces `rustc`-style error output with colored source snippets,
//! line numbers, underlined error locations, and formal 4-part Nyāya proofs.
//!
//! Defensively structured to prevent any panics on out-of-bounds spans,
//! missing source files, or mid-sequence multi-byte UTF-8 boundaries.

use crate::diagnostic::{Diagnostic, DiagnosticLevel};
use crate::span::{SourceFile, SourceId};
use std::collections::HashMap;

/// Collects diagnostics and renders them for the user.
pub struct DiagnosticEmitter {
    /// All emitted diagnostics.
    diagnostics: Vec<Diagnostic>,
    /// Source files for rendering snippets.
    sources: HashMap<SourceId, SourceFile>,
    /// Buffered rendered output for callers that need to capture diagnostics.
    rendered_output: String,
    /// Whether rendered diagnostics should also be mirrored to stderr.
    render_to_stderr: bool,
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
            rendered_output: String::new(),
            render_to_stderr: true,
            error_count: 0,
            warning_count: 0,
        }
    }

    /// Create an emitter that buffers rendered diagnostics instead of writing
    /// them directly to stderr.
    pub fn buffered() -> Self {
        Self {
            render_to_stderr: false,
            ..Self::new()
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

    /// Render a diagnostic.
    fn render(&mut self, diag: &Diagnostic) {
        // Level prefix with color
        let level_str = match diag.level {
            DiagnosticLevel::Error => "\x1b[1;31merror",
            DiagnosticLevel::Warning => "\x1b[1;33mwarning",
            DiagnosticLevel::Note => "\x1b[1;36mnote",
            DiagnosticLevel::Ice => "\x1b[1;31minternal compiler error",
        };

        // Error code
        let code_str = match &diag.code {
            Some(c) => format!("[{c}]"),
            None => String::new(),
        };

        self.render_line(&format!(
            "{}{}\x1b[1;37m: {}\x1b[0m",
            level_str, code_str, diag.message
        ));

        // Render each label defensively
        for label in &diag.labels {
            if label.span.is_dummy() {
                continue;
            }

            let mut lines_to_render = Vec::new();
            match self.sources.get(&label.span.source_id) {
                Some(source) => {
                    let val_span = source.validate_span(label.span);
                    let (line, col) = source.offset_to_line_col(val_span.start_byte as usize);
                    let source_path = source.path.clone();
                    let line_text = source.line_text(line).to_string();
                    let line_num = line + 1;
                    let col_num = col + 1;

                    // File location
                    lines_to_render.push(format!(
                        " \x1b[1;34m-->\x1b[0m {}:{}:{}",
                        source_path, line_num, col_num
                    ));

                    // Line number gutter width
                    let gutter_width = format!("{}", line_num).len();

                    // Empty gutter line
                    lines_to_render.push(format!(" {:>gutter_width$} \x1b[1;34m|\x1b[0m", ""));

                    // Source line
                    lines_to_render.push(format!(
                        " \x1b[1;34m{:>gutter_width$}\x1b[0m \x1b[1;34m|\x1b[0m {}",
                        line_num, line_text
                    ));

                    // Underline calculation based on character counts
                    let line_char_count = line_text.chars().count();
                    let safe_col = col.min(line_char_count);
                    let span_slice = val_span.slice(&source.source);
                    let span_char_len = span_slice.chars().count().max(1);
                    let underline_len = span_char_len
                        .min(line_char_count.saturating_sub(safe_col))
                        .max(1);

                    let padding = " ".repeat(safe_col);
                    let underline_char = if label.is_primary { '^' } else { '-' };
                    let color = if label.is_primary {
                        "\x1b[1;31m"
                    } else {
                        "\x1b[1;34m"
                    };
                    let underline =
                        std::iter::repeat_n(underline_char, underline_len).collect::<String>();

                    lines_to_render.push(format!(
                        " {:>gutter_width$} \x1b[1;34m|\x1b[0m {}{}{} {}\x1b[0m",
                        "", padding, color, underline, label.message
                    ));
                }
                None => {
                    // ICE-safe fallback label when source file is unregistered or missing
                    lines_to_render.push(format!(
                        " \x1b[1;34m-->\x1b[0m <unknown source #{}>:{}..{} \x1b[1;33m(source text unavailable)\x1b[0m",
                        label.span.source_id.0, label.span.start, label.span.end
                    ));
                    lines_to_render.push(format!("   \x1b[1;34m|\x1b[0m {}", label.message));
                }
            }

            for line in lines_to_render {
                self.render_line(&line);
            }
        }

        // Help text
        if let Some(help) = &diag.help {
            self.render_line(&format!(" \x1b[1;36mhelp\x1b[0m: {}", help));
        }

        // Note text
        if let Some(note) = &diag.note {
            self.render_line(&format!(" \x1b[1;36mnote\x1b[0m: {}", note));
        }

        // Formal Nyāya 4-part proof
        if let Some(proof) = &diag.proof {
            self.render_line(" \x1b[1;35m--- Nyāya 4-Part Proof (Nyāya-śāstra) ---\x1b[0m");
            self.render_line(&format!(
                "   \x1b[1;33m[Fact / Pratijñā]\x1b[0m   {}",
                proof.fact
            ));
            self.render_line(&format!(
                "   \x1b[1;31m[Reason / Hetu]\x1b[0m    {}",
                proof.reason
            ));
            if let Some(fix) = &proof.fix {
                self.render_line(&format!("   \x1b[1;32m[Fix / Udāharaṇa]\x1b[0m  {}", fix));
            }
            self.render_line(&format!(
                "   \x1b[1;36m[Law / Nigamana]\x1b[0m   {}",
                proof.law
            ));
        }

        self.render_line("");
    }

    fn render_line(&mut self, line: &str) {
        self.rendered_output.push_str(line);
        self.rendered_output.push('\n');
        if self.render_to_stderr {
            eprintln!("{line}");
        }
    }

    /// Return the currently buffered rendered output.
    pub fn rendered_output(&self) -> &str {
        &self.rendered_output
    }

    /// Take the buffered rendered output, leaving the emitter empty.
    pub fn take_rendered_output(&mut self) -> String {
        std::mem::take(&mut self.rendered_output)
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
    use crate::diagnostic::{Diagnostic, Label, NyayaProof};
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

        let diag = Diagnostic::error("E0001", "mismatched types").with_label(Label::primary(
            Span::new(SourceId(0), 14, 21),
            "expected `i32`, found `str`",
        ));

        emitter.emit(diag);
        assert!(emitter.has_errors());
    }

    #[test]
    fn test_buffered_emitter_captures_rendered_output() {
        let mut emitter = DiagnosticEmitter::buffered();
        emitter.add_source(SourceFile::new(
            SourceId(0),
            "test.agam".into(),
            "let x = y\n".into(),
        ));

        emitter.emit(
            Diagnostic::error("E0001", "unknown name")
                .with_label(Label::primary(Span::new(SourceId(0), 8, 9), "not found")),
        );

        let rendered = emitter.take_rendered_output();
        assert!(rendered.contains("unknown name"));
        assert!(rendered.contains("test.agam:1:9"));
        assert!(rendered.contains("not found"));
    }

    #[test]
    fn test_buffered_emitter_renders_nyaya_proof() {
        let mut emitter = DiagnosticEmitter::buffered();
        let proof = NyayaProof::new(
            "assignment target `x` is immutable",
            "cannot mutate value bound with immutable `let`",
            Some("change declaration to `let mut x`"),
            "variables in Agam are immutable by default",
        );
        let diag = Diagnostic::error("E0384", "cannot assign twice to immutable variable")
            .with_proof(proof);

        emitter.emit(diag);
        let rendered = emitter.take_rendered_output();
        assert!(rendered.contains("Nyāya 4-Part Proof"));
        assert!(rendered.contains("[Fact / Pratijñā]"));
        assert!(rendered.contains("assignment target `x` is immutable"));
        assert!(rendered.contains("[Reason / Hetu]"));
        assert!(rendered.contains("[Fix / Udāharaṇa]"));
        assert!(rendered.contains("change declaration to `let mut x`"));
        assert!(rendered.contains("[Law / Nigamana]"));
        assert!(rendered.contains("variables in Agam are immutable by default"));
    }

    #[test]
    fn test_missing_source_ice_safe_fallback() {
        let mut emitter = DiagnosticEmitter::buffered();
        // Label refers to SourceId(999) which is not in emitter.sources
        let diag = Diagnostic::error("E0999", "ghost file error").with_label(Label::primary(
            Span::new(SourceId(999), 10, 20),
            "unmapped span locus",
        ));

        emitter.emit(diag);
        let output = emitter.take_rendered_output();
        assert!(output.contains("<unknown source #999>:10..20"));
        assert!(output.contains("source text unavailable"));
        assert!(output.contains("unmapped span locus"));
    }

    #[test]
    fn test_multibyte_utf8_corrupt_span_rendering() {
        let mut emitter = DiagnosticEmitter::buffered();
        emitter.add_source(SourceFile::new(
            SourceId(1),
            "indic.agam".into(),
            "let x = \"नमस्ते Agam\";\n".into(),
        ));

        // Corrupted span pointing mid-byte into Devanagari sequence and past line end
        let diag = Diagnostic::error("E0100", "unicode boundary test").with_label(Label::primary(
            Span::new(SourceId(1), 10, 500),
            "invalid mid-byte slice",
        ));

        emitter.emit(diag);
        let output = emitter.take_rendered_output();
        assert!(output.contains("indic.agam:1:"));
        assert!(output.contains("नमस्ते Agam"));
        assert!(output.contains("invalid mid-byte slice"));
    }
}
