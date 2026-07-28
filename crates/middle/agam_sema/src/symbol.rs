//! Symbol table types.
//!
//! A `Symbol` represents a named entity (variable, function, type, module)
//! that has been declared in the program. Symbols carry resolved type
//! information and metadata used by later compiler passes.
//!
//! Enum variants carry full field info (`EnumVariantInfo`) to support
//! type-checked pattern matching, exhaustiveness checking, and
//! constructor resolution.

use agam_errors::Span;
use serde::{Deserialize, Serialize};

/// Unique identifier for a symbol within the compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(pub u32);

/// The kind of entity a symbol represents.
#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    /// A local or global variable (`let x`, `var y`, or implicit in dynamic mode).
    Variable { mutable: bool, ty: TypeId },
    /// A function declaration.
    Function {
        params: Vec<TypeId>,
        return_ty: TypeId,
        is_async: bool,
        generics: Vec<String>,
    },
    /// A struct type declaration.
    Struct { fields: Vec<(String, TypeId)> },
    /// An enum type declaration with full variant detail.
    Enum {
        variants: Vec<EnumVariantInfo>,
        generics: Vec<String>,
    },
    /// A trait declaration.
    Trait { methods: Vec<String> },
    /// A module.
    Module,
    /// A type alias.
    TypeAlias { target: TypeId },
    /// A constant value.
    Constant { ty: TypeId },
    /// A generic type parameter.
    TypeParam { bounds: Vec<TypeId> },
    /// A constraint shorthand (pratyāhāra): `constraint Sortable = Ord + Eq + Clone`.
    Constraint { bounds: Vec<TypeId> },
}

/// Unique identifier for an internal resolved type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypeId(pub u32);

/// A resolved symbol entry in the symbol table.
#[derive(Debug, Clone)]
pub struct Symbol {
    /// Unique ID for this symbol.
    pub id: SymbolId,
    /// The declared name of this symbol.
    pub name: String,
    /// What kind of entity this symbol is.
    pub kind: SymbolKind,
    /// Where this symbol was declared.
    pub span: Span,
    /// The scope depth at which this symbol lives (0 = global).
    pub depth: u32,
    /// Whether this symbol has been referenced (for dead-code warnings).
    pub used: bool,
}

/// Detailed information about an enum variant for type checking.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariantInfo {
    pub name: String,
    pub fields: VariantFieldKind,
}

/// The kind of fields an enum variant carries.
#[derive(Debug, Clone, PartialEq)]
pub enum VariantFieldKind {
    /// No fields: `None`, `Red`
    Unit,
    /// Positional fields: `Some(T)`, `Ok(T)`
    Tuple(Vec<TypeId>),
    /// Named fields: `Variant { x: i32, y: i32 }`
    Struct(Vec<(String, TypeId)>),
}
