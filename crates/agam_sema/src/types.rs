//! Internal type representation for semantic analysis.
//!
//! These are the *resolved* types used during type checking, distinct
//! from the AST's `TypeExpr` which is a syntactic representation.

use crate::symbol::{SymbolId, TypeId};

/// A resolved type in Agam's type system.
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    // ── Primitives ──

    /// Signed integers: i8, i16, i32, i64, i128, isize
    Int(IntSize),
    /// Unsigned integers: u8, u16, u32, u64, u128, usize
    UInt(IntSize),
    /// Floating-point: f32, f64
    Float(FloatSize),
    /// Boolean: bool
    Bool,
    /// Character: char
    Char,
    /// String: str (slice) or String (owned)
    Str,
    /// Unit type: () / void
    Unit,
    /// The never / bottom type: !
    Never,

    // ── Compound ──

    /// Array with known size: [T; N]
    Array { element: TypeId, size: usize },
    /// Slice: [T]
    Slice(TypeId),
    /// Tuple: (T, U, V)
    Tuple(Vec<TypeId>),
    /// Reference: &T or &mut T
    Ref { mutable: bool, inner: TypeId },
    /// Raw pointer: *T or *mut T
    Ptr { mutable: bool, inner: TypeId },
    /// Optional: T?
    Optional(TypeId),

    // ── Named / User-defined ──

    /// A named type referencing its symbol: struct, enum, type alias
    Named(SymbolId),
    /// A generic instantiation: Vec<i32>, HashMap<String, i32>
    Generic { base: TypeId, args: Vec<TypeId> },

    // ── Functions ──

    /// Function type: fn(A, B) -> C
    Function { params: Vec<TypeId>, ret: TypeId },

    // ── Trait Objects ──

    /// Dynamic trait object: dyn Trait
    DynTrait(SymbolId),

    // ── Inference ──

    /// A type variable (placeholder for inference): ?T0, ?T1, ...
    Var(u32),
    /// The universal dynamic type (runtime-checked, Python-like)
    Any,

    // ── Error ──

    /// Placeholder for types that failed to resolve (enables error recovery).
    Error,
}

/// Integer size variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntSize {
    I8, I16, I32, I64, I128, ISize,
}

/// Floating-point size variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatSize {
    F32, F64,
}

/// The type store — an arena that owns all resolved types.
///
/// Types are interned: each unique type gets exactly one `TypeId`.
/// This makes type comparison O(1) by ID instead of structural.
pub struct TypeStore {
    types: Vec<Type>,
}

impl TypeStore {
    pub fn new() -> Self {
        let mut store = Self { types: Vec::new() };
        // Pre-populate with well-known primitives so they have stable IDs.
        store.insert(Type::Unit);      // TypeId(0)
        store.insert(Type::Bool);      // TypeId(1)
        store.insert(Type::Char);      // TypeId(2)
        store.insert(Type::Str);       // TypeId(3)
        store.insert(Type::Int(IntSize::I32));   // TypeId(4) — default int
        store.insert(Type::Float(FloatSize::F64)); // TypeId(5) — default float
        store.insert(Type::Never);     // TypeId(6)
        store.insert(Type::Any);       // TypeId(7)
        store.insert(Type::Error);     // TypeId(8)
        store
    }

    /// Insert a new type, returning its ID.
    pub fn insert(&mut self, ty: Type) -> TypeId {
        // Simple linear scan for dedup; good enough for now.
        for (i, existing) in self.types.iter().enumerate() {
            if *existing == ty {
                return TypeId(i as u32);
            }
        }
        let id = TypeId(self.types.len() as u32);
        self.types.push(ty);
        id
    }

    /// Look up a type by ID.
    pub fn get(&self, id: TypeId) -> &Type {
        &self.types[id.0 as usize]
    }

    // ── Well-known type IDs ──

    pub fn unit(&self)  -> TypeId { TypeId(0) }
    pub fn bool(&self)  -> TypeId { TypeId(1) }
    pub fn char(&self)  -> TypeId { TypeId(2) }
    pub fn str(&self)   -> TypeId { TypeId(3) }
    pub fn i32(&self)   -> TypeId { TypeId(4) }
    pub fn f64(&self)   -> TypeId { TypeId(5) }
    pub fn never(&self) -> TypeId { TypeId(6) }
    pub fn any(&self)   -> TypeId { TypeId(7) }
    pub fn error(&self) -> TypeId { TypeId(8) }

    /// Create a fresh type variable for inference.
    pub fn fresh_var(&mut self) -> TypeId {
        let var_id = self.types.len() as u32;
        self.insert(Type::Var(var_id))
    }
}
