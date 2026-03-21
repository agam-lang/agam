//! Pattern AST nodes.
//!
//! Patterns destructure values in `let`, `match`, `for`, and function params.
//! Examples: `x`, `(a, b)`, `Some(value)`, `Point { x, y }`.

use crate::{Ident, NodeId, Path};
use crate::types::TypeExpr;
use agam_errors::Span;

/// A pattern node.
#[derive(Debug, Clone)]
pub struct Pattern {
    pub id: NodeId,
    pub span: Span,
    pub kind: PatternKind,
}

/// The different kinds of patterns.
#[derive(Debug, Clone)]
pub enum PatternKind {
    /// Wildcard: `_`
    Wildcard,

    /// Identifier binding: `x`, `mut x`
    Identifier {
        name: Ident,
        mutable: bool,
    },

    /// Literal: `42`, `"hello"`, `true`
    Literal(super::expr::Expr),

    /// Tuple destructure: `(a, b, c)`
    Tuple(Vec<Pattern>),

    /// Array/slice destructure: `[a, b, ..rest]`
    Array(Vec<Pattern>),

    /// Struct destructure: `Point { x, y }`
    Struct {
        path: Path,
        fields: Vec<FieldPattern>,
        rest: bool, // `..` at the end
    },

    /// Enum variant: `Some(value)`, `None`
    Variant {
        path: Path,
        fields: Vec<Pattern>,
    },

    /// Or pattern: `A | B | C`
    Or(Vec<Pattern>),

    /// Range pattern: `1..=10`
    Range {
        start: Box<Pattern>,
        end: Box<Pattern>,
        inclusive: bool,
    },

    /// Binding with a sub-pattern: `name @ Pattern`
    Binding {
        name: Ident,
        pattern: Box<Pattern>,
    },

    /// Rest pattern: `..` (in arrays and structs)
    Rest,

    /// Type-annotated pattern: `x: i32`
    Typed {
        pattern: Box<Pattern>,
        ty: TypeExpr,
    },
}

/// A field in a struct pattern.
#[derive(Debug, Clone)]
pub struct FieldPattern {
    pub name: Ident,
    pub pattern: Option<Pattern>,
    pub span: Span,
}
