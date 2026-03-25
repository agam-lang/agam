//! Pattern exhaustiveness checking for `match` expressions.
//!
//! Ensures that every possible value of the scrutinee type is covered
//! by at least one match arm. Reports:
//! - **Non-exhaustive patterns** — missing cases.
//! - **Unreachable patterns** — arms that can never match.

use std::collections::HashSet;
use agam_errors::Span;

/// A simplified pattern representation for exhaustiveness analysis.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SimplePattern {
    /// Wildcard `_` or variable binding — matches everything.
    Wildcard,
    /// A specific boolean value.
    Bool(bool),
    /// A specific integer literal.
    Int(i64),
    /// A specific string literal.
    Str(String),
    /// An enum variant by name.
    Variant(String),
    /// A tuple of patterns.
    Tuple(Vec<SimplePattern>),
    /// A constructor with fields (enum variant with data).
    Constructor { name: String, fields: Vec<SimplePattern> },
}

/// The type shape being matched against (needed to know what's exhaustive).
#[derive(Debug, Clone)]
pub enum TypeShape {
    /// Boolean — exactly two values.
    Bool,
    /// An enum with known variant names.
    Enum { variants: Vec<String> },
    /// Integer — unbounded, needs wildcard.
    Int,
    /// String — unbounded, needs wildcard.
    Str,
    /// Tuple of shapes.
    Tuple(Vec<TypeShape>),
    /// Any other type — needs wildcard for exhaustiveness.
    Other,
}

/// Error from exhaustiveness checking.
#[derive(Debug, Clone)]
pub struct ExhaustivenessError {
    pub message: String,
    pub span: Span,
}

/// Check if a set of patterns exhaustively covers a type shape.
pub fn check_exhaustiveness(
    patterns: &[SimplePattern],
    shape: &TypeShape,
    span: Span,
) -> Vec<ExhaustivenessError> {
    let mut errors = Vec::new();

    // If any pattern is a wildcard, it's automatically exhaustive.
    if patterns.iter().any(|p| matches!(p, SimplePattern::Wildcard)) {
        // Check for unreachable patterns after the wildcard.
        let mut found_wildcard = false;
        for (i, p) in patterns.iter().enumerate() {
            if found_wildcard {
                errors.push(ExhaustivenessError {
                    message: format!("unreachable pattern (arm {})", i + 1),
                    span,
                });
            }
            if matches!(p, SimplePattern::Wildcard) {
                found_wildcard = true;
            }
        }
        return errors;
    }

    match shape {
        TypeShape::Bool => {
            let covered: HashSet<bool> = patterns.iter().filter_map(|p| {
                if let SimplePattern::Bool(v) = p { Some(*v) } else { None }
            }).collect();

            if !covered.contains(&true) {
                errors.push(ExhaustivenessError {
                    message: "non-exhaustive pattern: missing `true`".into(),
                    span,
                });
            }
            if !covered.contains(&false) {
                errors.push(ExhaustivenessError {
                    message: "non-exhaustive pattern: missing `false`".into(),
                    span,
                });
            }
        }

        TypeShape::Enum { variants } => {
            let covered: HashSet<&str> = patterns.iter().filter_map(|p| match p {
                SimplePattern::Variant(name) => Some(name.as_str()),
                SimplePattern::Constructor { name, .. } => Some(name.as_str()),
                _ => None,
            }).collect();

            let missing: Vec<&str> = variants.iter()
                .filter(|v| !covered.contains(v.as_str()))
                .map(|v| v.as_str())
                .collect();

            if !missing.is_empty() {
                errors.push(ExhaustivenessError {
                    message: format!("non-exhaustive pattern: missing variant(s): {}", missing.join(", ")),
                    span,
                });
            }
        }

        TypeShape::Int | TypeShape::Str | TypeShape::Other => {
            // These types are unbounded — without a wildcard, they're never exhaustive.
            errors.push(ExhaustivenessError {
                message: "non-exhaustive pattern: missing wildcard `_` or default case".into(),
                span,
            });
        }

        TypeShape::Tuple(_shapes) => {
            // For tuples, each position must be independently exhaustive.
            // Simple check: if no wildcard exists, report non-exhaustive.
            errors.push(ExhaustivenessError {
                message: "non-exhaustive pattern: tuple match missing wildcard `_`".into(),
                span,
            });
        }
    }

    // Check for duplicate/unreachable patterns.
    let mut seen = HashSet::new();
    for (i, p) in patterns.iter().enumerate() {
        if !seen.insert(p) {
            errors.push(ExhaustivenessError {
                message: format!("unreachable pattern: duplicate arm {}", i + 1),
                span,
            });
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_span() -> Span { Span::dummy() }

    #[test]
    fn test_bool_exhaustive() {
        let patterns = vec![SimplePattern::Bool(true), SimplePattern::Bool(false)];
        let errors = check_exhaustiveness(&patterns, &TypeShape::Bool, dummy_span());
        assert!(errors.is_empty(), "errors: {:?}", errors);
    }

    #[test]
    fn test_bool_missing_true() {
        let patterns = vec![SimplePattern::Bool(false)];
        let errors = check_exhaustiveness(&patterns, &TypeShape::Bool, dummy_span());
        assert!(errors.iter().any(|e| e.message.contains("missing `true`")));
    }

    #[test]
    fn test_bool_missing_false() {
        let patterns = vec![SimplePattern::Bool(true)];
        let errors = check_exhaustiveness(&patterns, &TypeShape::Bool, dummy_span());
        assert!(errors.iter().any(|e| e.message.contains("missing `false`")));
    }

    #[test]
    fn test_enum_exhaustive() {
        let shape = TypeShape::Enum {
            variants: vec!["None".into(), "Some".into()],
        };
        let patterns = vec![
            SimplePattern::Variant("None".into()),
            SimplePattern::Variant("Some".into()),
        ];
        let errors = check_exhaustiveness(&patterns, &shape, dummy_span());
        assert!(errors.is_empty(), "errors: {:?}", errors);
    }

    #[test]
    fn test_enum_missing_variant() {
        let shape = TypeShape::Enum {
            variants: vec!["Red".into(), "Green".into(), "Blue".into()],
        };
        let patterns = vec![
            SimplePattern::Variant("Red".into()),
            SimplePattern::Variant("Green".into()),
        ];
        let errors = check_exhaustiveness(&patterns, &shape, dummy_span());
        assert!(errors.iter().any(|e| e.message.contains("Blue")));
    }

    #[test]
    fn test_wildcard_is_exhaustive() {
        let patterns = vec![SimplePattern::Wildcard];
        let errors = check_exhaustiveness(&patterns, &TypeShape::Int, dummy_span());
        assert!(errors.is_empty());
    }

    #[test]
    fn test_unreachable_after_wildcard() {
        let patterns = vec![
            SimplePattern::Wildcard,
            SimplePattern::Bool(true),
        ];
        let errors = check_exhaustiveness(&patterns, &TypeShape::Bool, dummy_span());
        assert!(errors.iter().any(|e| e.message.contains("unreachable")));
    }

    #[test]
    fn test_int_without_wildcard() {
        let patterns = vec![SimplePattern::Int(1), SimplePattern::Int(2)];
        let errors = check_exhaustiveness(&patterns, &TypeShape::Int, dummy_span());
        assert!(errors.iter().any(|e| e.message.contains("missing wildcard")));
    }

    #[test]
    fn test_duplicate_pattern() {
        let patterns = vec![
            SimplePattern::Bool(true),
            SimplePattern::Bool(true),
            SimplePattern::Bool(false),
        ];
        let errors = check_exhaustiveness(&patterns, &TypeShape::Bool, dummy_span());
        assert!(errors.iter().any(|e| e.message.contains("duplicate")));
    }
}
