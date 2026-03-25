//! Type inference engine — union-find based constraint solver.
//!
//! Implements a variant of Hindley-Milner type inference using:
//! 1. **Type variables** (`Type::Var`) as placeholders during inference.
//! 2. **Union-find** for efficient unification of type variables.
//! 3. **Constraint generation** during AST traversal.
//! 4. **Constraint solving** via unification.

use crate::symbol::TypeId;
use crate::types::{Type, TypeStore};

/// A type constraint: two types that must be equal.
#[derive(Debug, Clone)]
pub struct Constraint {
    pub expected: TypeId,
    pub actual: TypeId,
    /// Human-readable context for error messages.
    pub context: String,
}

/// Union-find structure for type variable unification.
///
/// Each type variable maps to either itself (a root) or another TypeId
/// (a forwarding link). `find()` follows the chain to the root.
pub struct UnionFind {
    /// parent[i] = the parent of TypeId(i). If parent[i] == i, it's a root.
    parent: Vec<u32>,
    /// rank[i] = tree depth heuristic for balancing.
    rank: Vec<u32>,
}

impl UnionFind {
    pub fn new(size: usize) -> Self {
        Self {
            parent: (0..size as u32).collect(),
            rank: vec![0; size],
        }
    }

    /// Ensure the union-find can hold at least `size` elements.
    pub fn grow(&mut self, size: usize) {
        while self.parent.len() < size {
            let id = self.parent.len() as u32;
            self.parent.push(id);
            self.rank.push(0);
        }
    }

    /// Find the root representative of a type variable (with path compression).
    pub fn find(&mut self, id: TypeId) -> TypeId {
        let i = id.0 as usize;
        if i >= self.parent.len() {
            self.grow(i + 1);
        }
        if self.parent[i] != id.0 {
            let root = self.find(TypeId(self.parent[i]));
            self.parent[i] = root.0; // path compression
            root
        } else {
            id
        }
    }

    /// Unify two type variables (union by rank).
    pub fn union(&mut self, a: TypeId, b: TypeId) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra == rb { return; }

        let (ra_i, rb_i) = (ra.0 as usize, rb.0 as usize);
        if self.rank[ra_i] < self.rank[rb_i] {
            self.parent[ra_i] = rb.0;
        } else if self.rank[ra_i] > self.rank[rb_i] {
            self.parent[rb_i] = ra.0;
        } else {
            self.parent[rb_i] = ra.0;
            self.rank[ra_i] += 1;
        }
    }
}

/// Inference error produced during unification.
#[derive(Debug, Clone)]
pub struct InferenceError {
    pub message: String,
    pub context: String,
}

/// The inference engine: collects constraints and solves them.
pub struct InferenceEngine {
    pub constraints: Vec<Constraint>,
    pub uf: UnionFind,
    pub errors: Vec<InferenceError>,
}

impl InferenceEngine {
    pub fn new(initial_capacity: usize) -> Self {
        Self {
            constraints: Vec::new(),
            uf: UnionFind::new(initial_capacity),
            errors: Vec::new(),
        }
    }

    /// Add a constraint: `expected` must unify with `actual`.
    pub fn constrain(&mut self, expected: TypeId, actual: TypeId, context: impl Into<String>) {
        self.constraints.push(Constraint {
            expected,
            actual,
            context: context.into(),
        });
    }

    /// Solve all collected constraints by unification.
    pub fn solve(&mut self, store: &TypeStore) {
        for constraint in self.constraints.clone() {
            if let Err(msg) = self.unify(constraint.expected, constraint.actual, store) {
                self.errors.push(InferenceError {
                    message: msg,
                    context: constraint.context,
                });
            }
        }
    }

    /// Unify two types. Returns `Ok(())` if they are compatible, `Err(message)` otherwise.
    fn unify(&mut self, a: TypeId, b: TypeId, store: &TypeStore) -> Result<(), String> {
        let ra = self.uf.find(a);
        let rb = self.uf.find(b);

        if ra == rb {
            return Ok(());
        }

        let ta = store.get(ra).clone();
        let tb = store.get(rb).clone();

        match (&ta, &tb) {
            // Type variables unify with anything.
            (Type::Var(_), _) => {
                self.uf.union(ra, rb);
                Ok(())
            }
            (_, Type::Var(_)) => {
                self.uf.union(rb, ra);
                Ok(())
            }

            // `Any` unifies with anything (dynamic typing).
            (Type::Any, _) | (_, Type::Any) => {
                self.uf.union(ra, rb);
                Ok(())
            }

            // Error type absorbs everything (error recovery).
            (Type::Error, _) | (_, Type::Error) => Ok(()),

            // Structural equality for primitives.
            (Type::Int(a), Type::Int(b)) if a == b => Ok(()),
            (Type::UInt(a), Type::UInt(b)) if a == b => Ok(()),
            (Type::Float(a), Type::Float(b)) if a == b => Ok(()),
            (Type::Bool, Type::Bool) => Ok(()),
            (Type::Char, Type::Char) => Ok(()),
            (Type::Str, Type::Str) => Ok(()),
            (Type::Unit, Type::Unit) => Ok(()),
            (Type::Never, _) | (_, Type::Never) => Ok(()), // Never is a subtype of everything.

            // References must match mutability and inner type.
            (Type::Ref { mutable: m1, inner: i1 }, Type::Ref { mutable: m2, inner: i2 }) => {
                if m1 != m2 {
                    return Err(format!("mutability mismatch: expected {}, found {}",
                        if *m1 { "&mut" } else { "&" },
                        if *m2 { "&mut" } else { "&" }));
                }
                self.unify(*i1, *i2, store)
            }

            // Pointers.
            (Type::Ptr { mutable: m1, inner: i1 }, Type::Ptr { mutable: m2, inner: i2 }) => {
                if m1 != m2 {
                    return Err("pointer mutability mismatch".into());
                }
                self.unify(*i1, *i2, store)
            }

            // Optionals.
            (Type::Optional(a), Type::Optional(b)) => self.unify(*a, *b, store),

            // Slices.
            (Type::Slice(a), Type::Slice(b)) => self.unify(*a, *b, store),

            // Arrays (size must match).
            (Type::Array { element: e1, size: s1 }, Type::Array { element: e2, size: s2 }) => {
                if s1 != s2 {
                    return Err(format!("array size mismatch: expected {}, found {}", s1, s2));
                }
                self.unify(*e1, *e2, store)
            }

            // Tuples (arity and element types must match).
            (Type::Tuple(a), Type::Tuple(b)) => {
                if a.len() != b.len() {
                    return Err(format!("tuple arity mismatch: expected {}, found {}", a.len(), b.len()));
                }
                for (x, y) in a.iter().zip(b.iter()) {
                    self.unify(*x, *y, store)?;
                }
                Ok(())
            }

            // Function types (param count, param types, and return type must match).
            (Type::Function { params: p1, ret: r1 }, Type::Function { params: p2, ret: r2 }) => {
                if p1.len() != p2.len() {
                    return Err(format!("function arity mismatch: expected {} params, found {}", p1.len(), p2.len()));
                }
                for (x, y) in p1.iter().zip(p2.iter()) {
                    self.unify(*x, *y, store)?;
                }
                self.unify(*r1, *r2, store)
            }

            // Named types must reference the same symbol.
            (Type::Named(a), Type::Named(b)) if a == b => Ok(()),

            // Generic instantiations: base and args must match.
            (Type::Generic { base: b1, args: a1 }, Type::Generic { base: b2, args: a2 }) => {
                self.unify(*b1, *b2, store)?;
                if a1.len() != a2.len() {
                    return Err("generic argument count mismatch".into());
                }
                for (x, y) in a1.iter().zip(a2.iter()) {
                    self.unify(*x, *y, store)?;
                }
                Ok(())
            }

            // Trait objects.
            (Type::DynTrait(a), Type::DynTrait(b)) if a == b => Ok(()),

            // Incompatible types.
            _ => Err(format!(
                "type mismatch: cannot unify {:?} with {:?}",
                ta, tb
            )),
        }
    }

    /// Resolve a TypeId to its final unified type (follows union-find chains).
    pub fn resolve(&mut self, id: TypeId) -> TypeId {
        self.uf.find(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_var_unifies_with_concrete() {
        let mut store = TypeStore::new();
        let var = store.fresh_var();
        let int = store.i32();

        let mut engine = InferenceEngine::new(store.i32().0 as usize + 10);
        engine.constrain(var, int, "test");
        engine.solve(&store);

        assert!(engine.errors.is_empty(), "errors: {:?}", engine.errors);
        let resolved = engine.resolve(var);
        // After unification, var should point to the same root as int
        let resolved_int = engine.resolve(int);
        assert_eq!(resolved, resolved_int);
    }

    #[test]
    fn test_concrete_type_mismatch() {
        let store = TypeStore::new();
        let int = store.i32();
        let boolean = store.bool();

        let mut engine = InferenceEngine::new(20);
        engine.constrain(int, boolean, "mismatch test");
        engine.solve(&store);

        assert_eq!(engine.errors.len(), 1);
        assert!(engine.errors[0].message.contains("type mismatch"));
    }

    #[test]
    fn test_two_vars_unify_transitively() {
        let mut store = TypeStore::new();
        let v1 = store.fresh_var();
        let v2 = store.fresh_var();
        let int = store.i32();

        let mut engine = InferenceEngine::new(20);
        engine.constrain(v1, v2, "v1 = v2");
        engine.constrain(v2, int, "v2 = i32");
        engine.solve(&store);

        assert!(engine.errors.is_empty());
        let r1 = engine.resolve(v1);
        let r2 = engine.resolve(v2);
        let ri = engine.resolve(int);
        assert_eq!(r1, r2);
        assert_eq!(r2, ri);
    }

    #[test]
    fn test_any_unifies_with_everything() {
        let store = TypeStore::new();
        let any = store.any();
        let int = store.i32();
        let boolean = store.bool();

        let mut engine = InferenceEngine::new(20);
        engine.constrain(any, int, "any = i32");
        engine.constrain(any, boolean, "any = bool");
        engine.solve(&store);

        assert!(engine.errors.is_empty());
    }

    #[test]
    fn test_function_type_unification() {
        let mut store = TypeStore::new();
        let int = store.i32();
        let boolean = store.bool();

        let fn1 = store.insert(Type::Function { params: vec![int], ret: boolean });
        let fn2 = store.insert(Type::Function { params: vec![int], ret: boolean });

        let mut engine = InferenceEngine::new(20);
        engine.constrain(fn1, fn2, "fn match");
        engine.solve(&store);

        assert!(engine.errors.is_empty());
    }

    #[test]
    fn test_function_arity_mismatch() {
        let mut store = TypeStore::new();
        let int = store.i32();
        let boolean = store.bool();

        let fn1 = store.insert(Type::Function { params: vec![int], ret: boolean });
        let fn2 = store.insert(Type::Function { params: vec![int, int], ret: boolean });

        let mut engine = InferenceEngine::new(20);
        engine.constrain(fn1, fn2, "arity mismatch");
        engine.solve(&store);

        assert_eq!(engine.errors.len(), 1);
        assert!(engine.errors[0].message.contains("arity mismatch"));
    }

    #[test]
    fn test_tuple_unification() {
        let mut store = TypeStore::new();
        let int = store.i32();
        let boolean = store.bool();

        let t1 = store.insert(Type::Tuple(vec![int, boolean]));
        let t2 = store.insert(Type::Tuple(vec![int, boolean]));

        let mut engine = InferenceEngine::new(20);
        engine.constrain(t1, t2, "tuple match");
        engine.solve(&store);

        assert!(engine.errors.is_empty());
    }

    #[test]
    fn test_ref_mutability_mismatch() {
        let mut store = TypeStore::new();
        let int = store.i32();

        let r1 = store.insert(Type::Ref { mutable: false, inner: int });
        let r2 = store.insert(Type::Ref { mutable: true, inner: int });

        let mut engine = InferenceEngine::new(20);
        engine.constrain(r1, r2, "ref mismatch");
        engine.solve(&store);

        assert_eq!(engine.errors.len(), 1);
        assert!(engine.errors[0].message.contains("mutability mismatch"));
    }
}
