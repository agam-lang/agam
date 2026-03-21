//! Expression AST nodes.
//!
//! Expressions are nodes that produce values. Examples:
//! `42`, `x + y`, `foo()`, `if cond { a } else { b }`.

use crate::{Ident, NodeId, Path};
use crate::types::TypeExpr;
use crate::pattern::Pattern;
use agam_errors::Span;

/// An expression node.
#[derive(Debug, Clone)]
pub struct Expr {
    pub id: NodeId,
    pub span: Span,
    pub kind: ExprKind,
}

/// The different kinds of expressions.
#[derive(Debug, Clone)]
pub enum ExprKind {
    // ── Literals ──

    /// Integer literal: `42`, `0xFF`, `0b1010`
    IntLiteral(i64),
    /// Float literal: `3.14`, `1.0e-5`
    FloatLiteral(f64),
    /// String literal: `"hello"`
    StringLiteral(String),
    /// Format string: `f"hello {name}"`
    FStringLiteral {
        parts: Vec<FStringPart>,
    },
    /// Boolean: `true` / `false`
    BoolLiteral(bool),
    /// Array literal: `[1, 2, 3]`
    ArrayLiteral(Vec<Expr>),
    /// Tuple literal: `(1, "a", true)`
    TupleLiteral(Vec<Expr>),

    // ── Names ──

    /// Variable / name reference: `x`, `my_var`
    Identifier(Ident),
    /// Path expression: `std::io::read`
    PathExpr(Path),

    // ── Operators ──

    /// Binary operation: `a + b`, `x == y`
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// Unary operation: `-x`, `!flag`, `&value`
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },

    // ── Access ──

    /// Field access: `obj.field`
    FieldAccess {
        object: Box<Expr>,
        field: Ident,
    },
    /// Index access: `arr[i]`
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },
    /// Method call: `obj.method(args)`
    MethodCall {
        object: Box<Expr>,
        method: Ident,
        args: Vec<Expr>,
    },

    // ── Calls ──

    /// Function call: `foo(a, b)`
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },

    // ── Control Flow (expression-position) ──

    /// If expression: `if cond { a } else { b }`
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
    },
    /// Match expression: `match x { ... }`
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    /// Block expression: `{ stmts; expr }`
    Block(Block),

    // ── Closures / Lambdas ──

    /// Lambda: `|x, y| x + y` or `|| { body }`
    Lambda {
        params: Vec<LambdaParam>,
        return_type: Option<Box<TypeExpr>>,
        body: Box<Expr>,
    },

    // ── Async / Await ──

    /// `await expr`
    Await(Box<Expr>),
    /// `spawn expr`
    Spawn(Box<Expr>),

    // ── Error propagation ──

    /// `expr?` — try/propagate error
    Try(Box<Expr>),

    // ── Assignment (expression in Agam) ──

    /// `x = value`
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
    },
    /// `x += value`, `x -= value`, etc.
    CompoundAssign {
        op: BinOp,
        target: Box<Expr>,
        value: Box<Expr>,
    },

    // ── Range ──

    /// `start..end` or `start..=end`
    Range {
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        inclusive: bool,
    },

    // ── Type ──

    /// Type cast: `expr as Type`
    Cast {
        expr: Box<Expr>,
        target_type: Box<TypeExpr>,
    },

    // ── Struct ──

    /// Struct literal: `Point { x: 1, y: 2 }`
    StructLiteral {
        path: Path,
        fields: Vec<FieldInit>,
    },
}

/// A part of an f-string.
#[derive(Debug, Clone)]
pub enum FStringPart {
    /// Raw text between interpolations.
    Literal(String),
    /// Interpolated expression: `{expr}`.
    Expr(Expr),
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    // Arithmetic
    Add,    // +
    Sub,    // -
    Mul,    // *
    Div,    // /
    Mod,    // %
    Pow,    // **

    // Comparison
    Eq,     // ==
    NotEq,  // !=
    Lt,     // <
    LtEq,   // <=
    Gt,     // >
    GtEq,   // >=

    // Logical
    And,    // &&
    Or,     // ||

    // Bitwise
    BitAnd, // &
    BitOr,  // |
    BitXor, // ^
    Shl,    // <<
    Shr,    // >>
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,    // -
    Not,    // !
    BitNot, // ~
    Ref,    // &
    Deref,  // *
}

/// A match arm: `Pattern => Expr`
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
    pub span: Span,
}

/// A block: sequence of statements, optional trailing expression.
#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<super::stmt::Stmt>,
    pub expr: Option<Box<Expr>>,
    pub span: Span,
}

/// Parameter in a lambda/closure.
#[derive(Debug, Clone)]
pub struct LambdaParam {
    pub name: Ident,
    pub ty: Option<TypeExpr>,
    pub span: Span,
}

/// A field initializer in a struct literal.
#[derive(Debug, Clone)]
pub struct FieldInit {
    pub name: Ident,
    pub value: Expr,
    pub span: Span,
}
