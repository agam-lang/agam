//! Statement AST nodes.
//!
//! Statements perform actions but don't produce values directly.
//! Examples: `let x = 5;`, `return 42;`, `while cond { ... }`.

use crate::expr::{Block, Expr};
use crate::pattern::Pattern;
use crate::types::TypeExpr;
use crate::{Ident, NodeId};
use agam_errors::Span;

/// A statement node.
#[derive(Debug, Clone)]
pub struct Stmt {
    pub id: NodeId,
    pub span: Span,
    pub kind: StmtKind,
}

/// The different kinds of statements.
#[derive(Debug, Clone)]
pub enum StmtKind {
    /// `let x = expr` or `let x: T = expr`
    Let {
        pattern: Pattern,
        ty: Option<TypeExpr>,
        value: Option<Expr>,
        mutable: bool,
    },

    /// `const X = expr` or `const X: T = expr`
    Const {
        name: Ident,
        ty: Option<TypeExpr>,
        value: Expr,
    },

    /// Expression statement: `expr;` or `expr` (trailing)
    Expression(Expr),

    /// `return expr`
    Return(Option<Expr>),

    /// `break expr`
    Break(Option<Expr>),

    /// `continue`
    Continue,

    /// `yield expr`
    Yield(Option<Expr>),

    /// `while condition { body }`
    While { condition: Expr, body: Block },

    /// `loop { body }`
    Loop { body: Block },

    /// `for pattern in iterable { body }`
    For {
        pattern: Pattern,
        iterable: Expr,
        body: Block,
    },

    /// `if cond { then } else if ... else { ... }`
    If {
        condition: Expr,
        then_branch: Block,
        else_branch: Option<ElseBranch>,
    },

    /// `match scrutinee { arms }`
    Match {
        scrutinee: Expr,
        arms: Vec<crate::expr::MatchArm>,
    },

    /// `try { body } catch E as e { handler }`
    TryCatch {
        body: Block,
        catches: Vec<CatchClause>,
    },

    /// `throw expr`
    Throw(Expr),

    /// A declaration used as a statement
    Declaration(crate::decl::Decl),
}

/// The else branch of an if statement.
#[derive(Debug, Clone)]
pub enum ElseBranch {
    /// `else { block }`
    Else(Block),
    /// `else if cond { ... }` (chains to another if)
    ElseIf(Box<Stmt>),
}

/// A catch clause in a try/catch statement.
#[derive(Debug, Clone)]
pub struct CatchClause {
    pub error_type: TypeExpr,
    pub binding: Option<Ident>,
    pub body: Block,
    pub span: Span,
}
