//! HIR node definitions.
//!
//! The HIR closely mirrors the AST but with:
//! - Resolved types on every expression.
//! - Desugared control flow.
//! - Unique IDs for every node.
//! - Generic parameters preserved for monomorphization.

use std::collections::HashMap;

use agam_sema::gpu::{GpuKernelConfig, GpuKernelParamAbi};
use agam_sema::symbol::TypeId;
use agam_sema::target::TargetProfile;

/// Unique HIR node identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HirId(pub u32);

/// A complete HIR module (one source file).
#[derive(Debug)]
pub struct HirModule {
    pub functions: Vec<HirFunction>,
    pub enum_layouts: HashMap<String, HirEnumLayout>,
    pub struct_layouts: HashMap<String, HirStructLayout>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HirEnumLayout {
    pub name: String,
    pub variants: Vec<HirEnumVariantLayout>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HirEnumVariantLayout {
    pub name: String,
    pub tag: u32,
    pub has_payload: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HirStructLayout {
    pub name: String,
    pub fields: Vec<String>,
}

/// A generic parameter in HIR (resolved from AST generic params).
#[derive(Debug, Clone)]
pub struct HirGenericParam {
    pub name: String,
    pub bounds: Vec<String>,
}

/// A HIR function.
#[derive(Debug)]
pub struct HirFunction {
    pub id: HirId,
    pub name: String,
    pub generics: Vec<HirGenericParam>,
    pub params: Vec<HirParam>,
    pub return_ty: TypeId,
    pub body: HirBlock,
    pub is_async: bool,
    /// Target deployment profile from `@target.*` annotations.
    pub target: TargetProfile,
    /// GPU kernel configuration from `@gpu(...)` annotations.
    pub gpu_config: Option<GpuKernelConfig>,
}

/// A function parameter in HIR.
#[derive(Debug)]
pub struct HirParam {
    pub name: String,
    pub ty: TypeId,
    pub mutable: bool,
    pub gpu_abi: GpuKernelParamAbi,
}

/// A block of statements with an optional trailing expression.
#[derive(Debug)]
pub struct HirBlock {
    pub stmts: Vec<HirStmt>,
    pub expr: Option<Box<HirExpr>>,
}

/// HIR statements — desugared, no syntactic sugar.
#[derive(Debug)]
pub enum HirStmt {
    /// Variable binding: `let x: T = expr`
    Let {
        name: String,
        ty: TypeId,
        value: Option<HirExpr>,
        mutable: bool,
    },
    /// Expression statement.
    Expr(HirExpr),
    /// Return from function.
    Return(Option<HirExpr>),
    /// While loop (for-in desugars to this).
    While { condition: HirExpr, body: HirBlock },
    /// Loop (infinite).
    Loop { body: HirBlock },
    /// If / else-if / else chain.
    If {
        condition: HirExpr,
        then_branch: HirBlock,
        else_branch: Option<HirBlock>,
    },
    /// Match expression lowered to statement.
    Match {
        scrutinee: HirExpr,
        arms: Vec<HirMatchArm>,
    },
    /// Break (with optional value).
    Break(Option<HirExpr>),
    /// Continue.
    Continue,
}

/// A match arm in HIR.
#[derive(Debug)]
pub struct HirMatchArm {
    pub pattern: HirPattern,
    pub guard: Option<HirExpr>,
    pub body: HirExpr,
}

/// Simplified pattern in HIR (after desugaring).
#[derive(Debug)]
pub enum HirPattern {
    /// Wildcard or variable binding.
    Wildcard,
    /// Variable binding with a name.
    Bind(String),
    /// Literal match.
    Literal(HirExpr),
    /// Tuple destructure.
    Tuple(Vec<HirPattern>),
    /// Enum variant: `Some(x)`, `None`
    Variant {
        name: String,
        fields: Vec<HirPattern>,
    },
    /// Struct destructure: `Point { x, y }`
    Struct {
        name: String,
        fields: Vec<(String, HirPattern)>,
    },
}

/// HIR expressions — all have a resolved type.
#[derive(Debug)]
pub struct HirExpr {
    pub id: HirId,
    pub ty: TypeId,
    pub kind: HirExprKind,
}

/// The kinds of HIR expressions.
#[derive(Debug)]
pub enum HirExprKind {
    // ── Literals ──
    IntLit(i64),
    FloatLit(f64),
    BoolLit(bool),
    StringLit(String),

    // ── Variables ──
    Var(String),

    // ── Binary / Unary ──
    Binary {
        op: HirBinOp,
        left: Box<HirExpr>,
        right: Box<HirExpr>,
    },
    Unary {
        op: HirUnaryOp,
        operand: Box<HirExpr>,
    },

    // ── Calls ──
    Call {
        callee: Box<HirExpr>,
        args: Vec<HirExpr>,
    },
    MethodCall {
        object: Box<HirExpr>,
        method: String,
        args: Vec<HirExpr>,
    },

    // ── Access ──
    FieldAccess {
        object: Box<HirExpr>,
        field: String,
    },
    Index {
        object: Box<HirExpr>,
        index: Box<HirExpr>,
    },

    // ── Assignment ──
    Assign {
        target: Box<HirExpr>,
        value: Box<HirExpr>,
    },

    // ── Aggregates ──
    Array(Vec<HirExpr>),
    Tuple(Vec<HirExpr>),

    // ── Struct / Enum construction ──
    /// Struct literal: `Point { x: 1, y: 2 }`
    StructLiteral {
        name: String,
        fields: Vec<(String, HirExpr)>,
    },
    /// Enum variant construction: `Some(42)`, `None`
    EnumVariant {
        enum_name: String,
        variant: String,
        fields: Vec<HirExpr>,
    },

    // ── Control flow ──
    Block(HirBlock),
    /// Match expression (produces a value).
    Match {
        scrutinee: Box<HirExpr>,
        arms: Vec<HirMatchArm>,
    },

    // ── Cast ──
    Cast {
        expr: Box<HirExpr>,
        target_ty: TypeId,
    },

    // ── Effects ──
    /// Perform an effect operation: `perform Effect.operation(args)`.
    Perform {
        effect: String,
        operation: String,
        args: Vec<HirExpr>,
    },
    /// Install an effect handler for a scoped block:
    /// `with handler handle Effect { body }`.
    HandleWith {
        effect: String,
        handler: String,
        body: Box<HirExpr>,
    },
    /// Allocate CUDA shared memory inside a GPU kernel.
    GpuSharedAlloc {
        element_abi: GpuKernelParamAbi,
        count: Box<HirExpr>,
    },
}

/// HIR binary operators (same as AST, kept separate for IR independence).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirBinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

/// HIR unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirUnaryOp {
    Neg,
    Not,
    BitNot,
    Ref,
    Deref,
}
