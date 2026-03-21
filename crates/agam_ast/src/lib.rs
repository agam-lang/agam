//! # agam_ast
//!
//! Abstract Syntax Tree definitions for the Agam language.
//! Both `@lang.base` and `@lang.advance` modes produce nodes from this unified AST.
//!
//! The AST is organized into four main categories:
//! - [`Expr`] — Expressions (produce values)
//! - [`Stmt`] — Statements (perform actions)
//! - [`Decl`] — Declarations (introduce names)
//! - [`TypeExpr`] — Type expressions (describe types)
//! - [`Pattern`] — Patterns (destructure values)

pub mod expr;
pub mod stmt;
pub mod decl;
pub mod types;
pub mod pattern;
pub mod visitor;
pub mod pretty;

use agam_errors::Span;

/// Unique identifier for an AST node (used for later analysis passes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u32);

/// A complete Agam source file.
#[derive(Debug, Clone)]
pub struct Module {
    pub id: NodeId,
    pub span: Span,
    /// Top-level declarations in the file.
    pub declarations: Vec<decl::Decl>,
}

/// Identifier with span information.
#[derive(Debug, Clone, PartialEq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

impl Ident {
    pub fn new(name: impl Into<String>, span: Span) -> Self {
        Self { name: name.into(), span }
    }
}

/// A path like `std::collections::HashMap` or `agam.io.File`.
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub segments: Vec<Ident>,
    pub span: Span,
}
