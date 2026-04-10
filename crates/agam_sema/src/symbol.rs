//! Symbol table types.
//!
//! A `Symbol` represents a named entity (variable, function, type, module)
//! that has been declared in the program. Symbols carry resolved type
//! information and metadata used by later compiler passes.

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
    },
    /// A struct type declaration.
    Struct { fields: Vec<(String, TypeId)> },
    /// An enum type declaration.
    Enum { variants: Vec<String> },
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
