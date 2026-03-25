//! Type expression AST nodes.
//!
//! Agam supports a **dual typing system**:
//! - **Static typing** (`let x: i32 = 42`) — checked at compile time
//! - **Dynamic typing** (`var x = 42`) — checked at runtime, Python-like
//!
//! Examples: `i32`, `Vec<T>`, `fn(i32) -> bool`, `&str`, `dyn Any`.

use crate::{NodeId, Path};
use agam_errors::Span;

/// Typing mode — Agam's dual type system.
///
/// ```agam
/// # Static (compile-time checked):
/// let x: i32 = 42
/// let name: String = "hello"
///
/// # Dynamic (runtime checked, like Python):
/// var x = 42          # type inferred at runtime
/// var x: dyn = 42     # explicitly dynamic
///
/// # Inferred static (compiler determines the type):
/// let x = 42          # compiler infers i32
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeMode {
    /// Compile-time type checking (default for `let` with type annotation).
    Static,
    /// Runtime type checking (used with `var` or `dyn`).
    Dynamic,
    /// Compiler infers the type and mode (default for `let` without annotation).
    Inferred,
}

impl Default for TypeMode {
    fn default() -> Self {
        TypeMode::Inferred
    }
}

/// A type expression node.
#[derive(Debug, Clone)]
pub struct TypeExpr {
    pub id: NodeId,
    pub span: Span,
    pub kind: TypeExprKind,
    /// Whether this type is statically or dynamically checked.
    pub mode: TypeMode,
}

/// The different kinds of type expressions.
#[derive(Debug, Clone)]
pub enum TypeExprKind {
    /// A named type: `i32`, `String`, `MyStruct`
    Named(Path),

    /// Generic type: `Vec<T>`, `HashMap<K, V>`
    Generic {
        base: Path,
        args: Vec<TypeExpr>,
    },

    /// Array type: `[T; N]`
    Array {
        element: Box<TypeExpr>,
        size: Option<Box<super::expr::Expr>>,
    },

    /// Slice type: `[T]`
    Slice(Box<TypeExpr>),

    /// Tuple type: `(T, U, V)`
    Tuple(Vec<TypeExpr>),

    /// Reference type: `&T` or `&mut T`
    Reference {
        mutable: bool,
        inner: Box<TypeExpr>,
    },

    /// Pointer type: `*T` or `*mut T`
    Pointer {
        mutable: bool,
        inner: Box<TypeExpr>,
    },

    /// Function type: `fn(A, B) -> C`
    Function {
        params: Vec<TypeExpr>,
        return_type: Box<TypeExpr>,
    },

    /// Optional/nullable: `T?` or `Option<T>`
    Optional(Box<TypeExpr>),

    /// Result type: `Result<T, E>`
    Result {
        ok: Box<TypeExpr>,
        err: Box<TypeExpr>,
    },

    /// `Self` type
    SelfType,

    /// Inferred type (placeholder for type inference): `_`
    Inferred,

    /// `never` / `!` — the bottom type
    Never,

    /// Dynamic type: `dyn` — type is checked at runtime (Python-like).
    /// `var x: dyn = anything`
    Dynamic,

    /// Refinement type: `{ base | predicate }`.
    /// E.g., `{v: i32 | v > 0}`
    Refined {
        base: Box<TypeExpr>,
        predicate: Box<super::expr::Expr>,
    },

    /// `dyn Trait` — a trait object (dynamically dispatched).
    DynTrait(Box<TypeExpr>),

    /// `Any` — the universal dynamic type (accepts any value at runtime).
    Any,
}

/// Built-in primitive type names for quick matching.
pub fn is_primitive_type(name: &str) -> bool {
    matches!(
        name,
        "i8" | "i16" | "i32" | "i64" | "i128" | "isize"
            | "u8" | "u16" | "u32" | "u64" | "u128" | "usize"
            | "f32" | "f64"
            | "bool"
            | "char"
            | "str"
            | "String"
            | "void"
            | "never"
            | "Any"
    )
}

