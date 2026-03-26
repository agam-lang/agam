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
        store.insert(Type::Int(IntSize::I8));    // TypeId(9)
        store.insert(Type::Int(IntSize::I16));   // TypeId(10)
        store.insert(Type::Int(IntSize::I64));   // TypeId(11)
        store.insert(Type::Int(IntSize::I128));  // TypeId(12)
        store.insert(Type::Int(IntSize::ISize)); // TypeId(13)
        store.insert(Type::UInt(IntSize::I8));   // TypeId(14)
        store.insert(Type::UInt(IntSize::I16));  // TypeId(15)
        store.insert(Type::UInt(IntSize::I32));  // TypeId(16)
        store.insert(Type::UInt(IntSize::I64));  // TypeId(17)
        store.insert(Type::UInt(IntSize::I128)); // TypeId(18)
        store.insert(Type::UInt(IntSize::ISize)); // TypeId(19)
        store.insert(Type::Float(FloatSize::F32)); // TypeId(20)
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
    pub fn i8(&self)    -> TypeId { TypeId(9) }
    pub fn i16(&self)   -> TypeId { TypeId(10) }
    pub fn i64(&self)   -> TypeId { TypeId(11) }
    pub fn i128(&self)  -> TypeId { TypeId(12) }
    pub fn isize(&self) -> TypeId { TypeId(13) }
    pub fn u8(&self)    -> TypeId { TypeId(14) }
    pub fn u16(&self)   -> TypeId { TypeId(15) }
    pub fn u32(&self)   -> TypeId { TypeId(16) }
    pub fn u64(&self)   -> TypeId { TypeId(17) }
    pub fn u128(&self)  -> TypeId { TypeId(18) }
    pub fn usize(&self) -> TypeId { TypeId(19) }
    pub fn f32(&self)   -> TypeId { TypeId(20) }

    /// Create a fresh type variable for inference.
    pub fn fresh_var(&mut self) -> TypeId {
        let var_id = self.types.len() as u32;
        self.insert(Type::Var(var_id))
    }
}

pub fn builtin_type_id_for_name(types: &TypeStore, name: &str) -> Option<TypeId> {
    match name {
        "i8" => Some(types.i8()),
        "i16" => Some(types.i16()),
        "i32" => Some(types.i32()),
        "i64" => Some(types.i64()),
        "i128" => Some(types.i128()),
        "isize" => Some(types.isize()),
        "u8" => Some(types.u8()),
        "u16" => Some(types.u16()),
        "u32" => Some(types.u32()),
        "u64" => Some(types.u64()),
        "u128" => Some(types.u128()),
        "usize" => Some(types.usize()),
        "f32" => Some(types.f32()),
        "f64" => Some(types.f64()),
        "bool" => Some(types.bool()),
        "char" => Some(types.char()),
        "str" | "String" => Some(types.str()),
        "void" => Some(types.unit()),
        "never" => Some(types.never()),
        "Any" => Some(types.any()),
        _ => None,
    }
}

pub fn builtin_type_by_id(id: TypeId) -> Option<Type> {
    match id.0 {
        0 => Some(Type::Unit),
        1 => Some(Type::Bool),
        2 => Some(Type::Char),
        3 => Some(Type::Str),
        4 => Some(Type::Int(IntSize::I32)),
        5 => Some(Type::Float(FloatSize::F64)),
        6 => Some(Type::Never),
        7 => Some(Type::Any),
        8 => Some(Type::Error),
        9 => Some(Type::Int(IntSize::I8)),
        10 => Some(Type::Int(IntSize::I16)),
        11 => Some(Type::Int(IntSize::I64)),
        12 => Some(Type::Int(IntSize::I128)),
        13 => Some(Type::Int(IntSize::ISize)),
        14 => Some(Type::UInt(IntSize::I8)),
        15 => Some(Type::UInt(IntSize::I16)),
        16 => Some(Type::UInt(IntSize::I32)),
        17 => Some(Type::UInt(IntSize::I64)),
        18 => Some(Type::UInt(IntSize::I128)),
        19 => Some(Type::UInt(IntSize::ISize)),
        20 => Some(Type::Float(FloatSize::F32)),
        _ => None,
    }
}
