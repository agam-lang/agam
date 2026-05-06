//! Declaration AST nodes.
//!
//! Declarations introduce new names into scope: functions, structs,
//! enums, traits, impls, modules, imports.

use crate::expr::{Block, Expr};
use crate::pattern::Pattern;
use crate::types::TypeExpr;
use crate::{Ident, NodeId, Path};
use agam_errors::Span;

/// A compiler attribute, e.g. `#[align(64)]`, `#[dispatch(SIMD)]`.
#[derive(Debug, Clone)]
pub struct Attribute {
    pub name: String,
    pub args: Vec<String>,
    pub span: Span,
}

impl Attribute {
    /// Check if this attribute has the given name.
    pub fn is(&self, name: &str) -> bool {
        self.name == name
    }

    /// Get the first argument, if any.
    pub fn first_arg(&self) -> Option<&str> {
        self.args.first().map(|s| s.as_str())
    }
}

/// A declaration node.
#[derive(Debug, Clone)]
pub struct Decl {
    pub id: NodeId,
    pub span: Span,
    pub kind: DeclKind,
    /// Compiler attributes: `#[align(64)]`, `#[dispatch(SIMD)]`, etc.
    pub attributes: Vec<Attribute>,
}

/// The different kinds of declarations.
#[derive(Debug, Clone)]
pub enum DeclKind {
    /// Function declaration.
    Function(FunctionDecl),

    /// Struct declaration.
    Struct(StructDecl),

    /// Enum declaration.
    Enum(EnumDecl),

    /// Trait declaration.
    Trait(TraitDecl),

    /// Impl block.
    Impl(ImplDecl),

    /// Module declaration: `mod name { ... }` or `mod name;`
    Module(ModuleDecl),

    /// Use/import statement: `use path::to::item`
    Use(UseDecl),

    /// Type alias: `type Name = ExistingType`
    TypeAlias {
        name: Ident,
        generics: Vec<GenericParam>,
        ty: TypeExpr,
        visibility: Visibility,
    },

    /// Effect declaration: `effect IO { fn read() -> String; fn write(s: String); }`
    Effect(EffectDecl),

    /// Handler: `handle io_handler for IO { fn read() -> String: resume("hello") }`
    Handler(HandlerDecl),
}

/// Visibility modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// Private (default).
    Private,
    /// Public: `pub`.
    Public,
}

/// Function declaration.
#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub params: Vec<FunctionParam>,
    pub return_type: Option<TypeExpr>,
    pub body: Option<Block>,
    pub visibility: Visibility,
    pub is_async: bool,
    pub annotations: Vec<Annotation>,
    pub span: Span,
}

/// A function parameter.
#[derive(Debug, Clone)]
pub struct FunctionParam {
    pub pattern: Pattern,
    pub ty: TypeExpr,
    pub default: Option<Expr>,
    pub span: Span,
}

/// Struct declaration.
#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub fields: Vec<FieldDecl>,
    pub visibility: Visibility,
    pub annotations: Vec<Annotation>,
    pub span: Span,
}

/// A struct field declaration.
#[derive(Debug, Clone)]
pub struct FieldDecl {
    pub name: Ident,
    pub ty: TypeExpr,
    pub default: Option<Expr>,
    pub visibility: Visibility,
    pub span: Span,
}

/// Enum declaration.
#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub variants: Vec<EnumVariant>,
    pub visibility: Visibility,
    pub span: Span,
}

/// An enum variant.
#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: Ident,
    pub fields: VariantFields,
    pub span: Span,
}

/// Fields of an enum variant.
#[derive(Debug, Clone)]
pub enum VariantFields {
    /// No fields: `None`
    Unit,
    /// Tuple-like: `Some(T)`
    Tuple(Vec<TypeExpr>),
    /// Struct-like: `Variant { x: i32, y: i32 }`
    Struct(Vec<FieldDecl>),
}

/// Trait declaration.
#[derive(Debug, Clone)]
pub struct TraitDecl {
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub super_traits: Vec<TypeExpr>,
    pub items: Vec<TraitItem>,
    pub visibility: Visibility,
    pub span: Span,
}

/// An item within a trait.
#[derive(Debug, Clone)]
pub enum TraitItem {
    /// Method signature (possibly with default body).
    Method(FunctionDecl),
    /// Associated type: `type Item;`
    AssociatedType {
        name: Ident,
        bounds: Vec<TypeExpr>,
        default: Option<TypeExpr>,
    },
}

/// Impl block.
#[derive(Debug, Clone)]
pub struct ImplDecl {
    pub generics: Vec<GenericParam>,
    /// The trait being implemented (None for inherent impls).
    pub trait_path: Option<Path>,
    /// The type being implemented for.
    pub target_type: TypeExpr,
    pub items: Vec<Decl>,
    pub span: Span,
}

/// Module declaration.
#[derive(Debug, Clone)]
pub struct ModuleDecl {
    pub name: Ident,
    pub body: Option<Vec<Decl>>,
    pub visibility: Visibility,
    pub span: Span,
}

/// Use/import declaration.
#[derive(Debug, Clone)]
pub struct UseDecl {
    pub path: Path,
    pub alias: Option<Ident>,
    pub items: Option<Vec<UseItem>>,
    pub visibility: Visibility,
    pub span: Span,
}

/// An item in a use declaration: `use path::{A, B as C}`.
#[derive(Debug, Clone)]
pub struct UseItem {
    pub name: Ident,
    pub alias: Option<Ident>,
    pub span: Span,
}

/// Generic type parameter: `T`, `T: Trait`, `T: Trait + Other`.
#[derive(Debug, Clone)]
pub struct GenericParam {
    pub name: Ident,
    pub bounds: Vec<TypeExpr>,
    pub default: Option<TypeExpr>,
    pub span: Span,
}

/// An annotation: `@safe`, `@gpu`, `@hot_reload`, `@test`, etc.
#[derive(Debug, Clone)]
pub struct Annotation {
    pub name: Ident,
    pub args: Vec<Expr>,
    pub span: Span,
}

/// Effect declaration: `effect IO { fn read() -> String; fn write(s: String); }`
#[derive(Debug, Clone)]
pub struct EffectDecl {
    pub name: Ident,
    pub operations: Vec<EffectOp>,
    pub visibility: Visibility,
    pub span: Span,
}

/// A single operation within an effect.
#[derive(Debug, Clone)]
pub struct EffectOp {
    pub name: Ident,
    pub params: Vec<(Ident, TypeExpr)>,
    pub return_type: Option<TypeExpr>,
    pub span: Span,
}

/// Handler declaration: `handle name for EffectName { ... }`
#[derive(Debug, Clone)]
pub struct HandlerDecl {
    pub name: Ident,
    pub effect_name: Ident,
    pub clauses: Vec<HandlerClause>,
    pub span: Span,
}

/// A single handler clause implementing an effect operation.
#[derive(Debug, Clone)]
pub struct HandlerClause {
    pub op_name: Ident,
    pub params: Vec<Ident>,
    pub body: Expr,
    pub span: Span,
}
