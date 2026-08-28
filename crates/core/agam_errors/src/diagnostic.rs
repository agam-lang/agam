//! Diagnostic types for compiler error/warning/info messages.
//!
//! Each diagnostic carries a severity level, a message, and zero or more
//! [`Label`]s that highlight specific source locations.

use crate::span::Span;
use std::fmt;

/// Severity level of a diagnostic message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagnosticLevel {
    /// Informational note (does not prevent compilation).
    Note,
    /// Warning (compilation continues, but user should investigate).
    Warning,
    /// Error (compilation will fail).
    Error,
    /// Internal compiler error (a bug in agamc itself).
    Ice,
}

impl fmt::Display for DiagnosticLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiagnosticLevel::Note => write!(f, "note"),
            DiagnosticLevel::Warning => write!(f, "warning"),
            DiagnosticLevel::Error => write!(f, "error"),
            DiagnosticLevel::Ice => write!(f, "internal compiler error"),
        }
    }
}

#[cfg(test)]
#[test]
fn test_diagnostic_level_display() {
    assert_eq!(DiagnosticLevel::Error.to_string(), "error");
    assert_eq!(DiagnosticLevel::Warning.to_string(), "warning");
    assert_eq!(DiagnosticLevel::Note.to_string(), "note");
    assert_eq!(DiagnosticLevel::Ice.to_string(), "internal compiler error");
}

/// Look up human-readable explanation and formal Nyāya grounding for an error code.
pub fn explain_code(code: &str) -> Option<&'static str> {
    match code.trim().to_uppercase().as_str() {
        "E0001" | "E001" => Some(
            "E0001: Type Mismatch\n  • Fact: Expression produces type T1 where T2 was expected.\n  • Reason: Agam requires explicit type conversions to prevent silent precision loss.\n  • Fix: Cast the expression using `as T2` or match the expected function signature.\n  • Law: Type Soundness (Sandhi Lattice Invariant).",
        ),
        "E0010" | "E010" => Some(
            "E0010: Unresolved Identifier\n  • Fact: Symbol name not found in the active lexical scope.\n  • Reason: Variable, function, or trait was not declared or imported before use.\n  • Fix: Declare the identifier with `let` or import the defining module.\n  • Law: Lexical Scope Resolution Invariant.",
        ),
        "E0034" | "E034" => Some(
            "E0034: Borrow-Check / Use After Move Violation\n  • Fact: Value accessed after ownership was moved to another scope or callee.\n  • Reason: Linear/affine ownership prevents multiple exclusive owners of heap resources.\n  • Fix: Clone the value before moving, or pass by shared borrow `&T`.\n  • Law: Zero-Aliasing Memory Safety Guarantee.",
        ),
        _ => None,
    }
}

/// A label attached to a diagnostic, highlighting a specific source location.
#[derive(Debug, Clone)]
pub struct Label {
    /// The source span this label points to.
    pub span: Span,
    /// A message describing what's wrong at this location.
    pub message: String,
    /// Whether this is the primary label (vs. a secondary/context label).
    pub is_primary: bool,
}

impl Label {
    /// Create a primary label (the main error location).
    pub fn primary(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            is_primary: true,
        }
    }

    /// Create a secondary label (additional context).
    pub fn secondary(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            is_primary: false,
        }
    }
}

/// A unique error code for each class of diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ErrorCode(pub &'static str);

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A formal 4-part Nyāya proof (*Fact, Reason, Fix, Law*) for a compiler diagnostic.
///
/// Grounded in classical Indian epistemology (Nyāya-śāstra):
/// 1. **Pratijñā (Fact)**: The thesis or empirical condition observed at the code locus.
/// 2. **Hetu (Reason)**: The underlying formal semantic/type/effect invariant violation.
/// 3. **Udāharaṇa (Fix/Example)**: The actionable code modification / suggestion.
/// 4. **Nigamana (Law/Conclusion)**: The universal language specification axiom or constraint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NyayaProof {
    /// Fact / Pratijñā: The empirical condition observed at the code locus.
    pub fact: String,
    /// Reason / Hetu: The formal semantic/type/effect invariant violated.
    pub reason: String,
    /// Fix / Udāharaṇa: The actionable fix or example correcting the violation.
    pub fix: Option<String>,
    /// Law / Nigamana: The governing language rule or invariant constraint.
    pub law: String,
}

impl NyayaProof {
    /// Construct a new formal 4-part Nyāya proof.
    pub fn new(
        fact: impl Into<String>,
        reason: impl Into<String>,
        fix: Option<impl Into<String>>,
        law: impl Into<String>,
    ) -> Self {
        Self {
            fact: fact.into(),
            reason: reason.into(),
            fix: fix.map(Into::into),
            law: law.into(),
        }
    }
}

/// A compiler diagnostic: an error, warning, or note with source locations.
///
/// # Example
///
/// ```ignore
/// Diagnostic::error("E0001", "mismatched types")
///     .with_label(Label::primary(span, "expected `i32`, found `str`"))
///     .with_label(Label::secondary(other_span, "required by this binding"))
///     .with_help("consider converting with `.parse::<i32>()`")
/// ```
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Severity level.
    pub level: DiagnosticLevel,
    /// Error code (e.g., "E0001").
    pub code: Option<ErrorCode>,
    /// Primary message.
    pub message: String,
    /// Labels pointing to source locations.
    pub labels: Vec<Label>,
    /// Optional help text suggesting a fix.
    pub help: Option<String>,
    /// Optional longer explanation of the error.
    pub note: Option<String>,
    /// Optional formal 4-part Nyāya proof.
    pub proof: Option<NyayaProof>,
}

impl Diagnostic {
    /// Create an error diagnostic.
    pub fn error(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            level: DiagnosticLevel::Error,
            code: Some(ErrorCode(code)),
            message: message.into(),
            labels: Vec::new(),
            help: None,
            note: None,
            proof: None,
        }
    }

    /// Create a warning diagnostic.
    pub fn warning(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            level: DiagnosticLevel::Warning,
            code: Some(ErrorCode(code)),
            message: message.into(),
            labels: Vec::new(),
            help: None,
            note: None,
            proof: None,
        }
    }

    /// Create a note diagnostic (no error code).
    pub fn note(message: impl Into<String>) -> Self {
        Self {
            level: DiagnosticLevel::Note,
            code: None,
            message: message.into(),
            labels: Vec::new(),
            help: None,
            note: None,
            proof: None,
        }
    }

    /// Create an internal compiler error diagnostic.
    pub fn ice(message: impl Into<String>) -> Self {
        Self {
            level: DiagnosticLevel::Ice,
            code: None,
            message: message.into(),
            labels: Vec::new(),
            help: None,
            note: None,
            proof: None,
        }
    }

    /// Add a label to this diagnostic.
    pub fn with_label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }

    /// Add help text.
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Add a note.
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// Add a formal 4-part Nyāya proof.
    pub fn with_proof(mut self, proof: NyayaProof) -> Self {
        self.proof = Some(proof);
        self
    }

    /// Check if this diagnostic is an error or ICE.
    pub fn is_error(&self) -> bool {
        matches!(self.level, DiagnosticLevel::Error | DiagnosticLevel::Ice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::{SourceId, Span};

    #[test]
    fn test_error_creation() {
        let diag = Diagnostic::error("E0001", "mismatched types")
            .with_label(Label::primary(
                Span::new(SourceId(0), 10, 20),
                "expected `i32`, found `str`",
            ))
            .with_help("try `.parse::<i32>()`");

        assert!(diag.is_error());
        assert_eq!(diag.code.unwrap().0, "E0001");
        assert_eq!(diag.labels.len(), 1);
        assert!(diag.labels[0].is_primary);
        assert_eq!(diag.help.as_deref(), Some("try `.parse::<i32>()`"));
    }

    #[test]
    fn test_warning_is_not_error() {
        let diag = Diagnostic::warning("W0001", "unused variable");
        assert!(!diag.is_error());
    }

    #[test]
    fn test_ice_is_error() {
        let diag = Diagnostic::ice("assertion failed in type checker");
        assert!(diag.is_error());
        assert_eq!(diag.level, DiagnosticLevel::Ice);
    }

    #[test]
    fn test_nyaya_proof_creation() {
        let proof = NyayaProof::new(
            "expression evaluated to type `str`",
            "function return contract expects type `i32`",
            Some("use `.parse::<i32>()` or cast expression"),
            "every returned expression must strictly match function signature",
        );
        let diag = Diagnostic::error("E0308", "mismatched types").with_proof(proof.clone());
        assert_eq!(diag.proof, Some(proof));
    }
}
