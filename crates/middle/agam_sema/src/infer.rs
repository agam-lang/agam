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

    /// Bind a type variable to point to another type.
    pub fn bind(&mut self, var: TypeId, target: TypeId) {
        let r_var = self.find(var);
        let r_target = self.find(target);
        if r_var != r_target {
            let var_idx = r_var.0 as usize;
            if var_idx >= self.parent.len() {
                self.grow(var_idx + 1);
            }
            self.parent[var_idx] = r_target.0;
        }
    }

    /// Unify two type variables (union by rank).
    pub fn union(&mut self, a: TypeId, b: TypeId) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra == rb {
            return;
        }

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
            // Type variables unify with anything (with occurs-check protection).
            (Type::Var(_), Type::Var(_)) => {
                self.uf.bind(ra, rb);
                Ok(())
            }
            (Type::Var(_), _) => {
                if self.occurs_in(ra, rb, store) {
                    return Err(
                        "cyclic type detected during unification (occurs check failed)".into(),
                    );
                }
                self.uf.bind(ra, rb);
                Ok(())
            }
            (_, Type::Var(_)) => {
                if self.occurs_in(rb, ra, store) {
                    return Err(
                        "cyclic type detected during unification (occurs check failed)".into(),
                    );
                }
                self.uf.bind(rb, ra);
                Ok(())
            }

            // `Any` unifies with anything (dynamic typing).
            (Type::Any, _) | (_, Type::Any) => {
                self.uf.union(ra, rb);
                Ok(())
            }

            // Error type absorbs everything (error recovery).
            (Type::Error, _) | (_, Type::Error) => Ok(()),

            // Structural equality for primitives and integer widening.
            (Type::Int(a), Type::Int(b)) => {
                if a == b {
                    Ok(())
                } else {
                    let wider = if a.bits() >= b.bits() { ra } else { rb };
                    self.uf.bind(ra, wider);
                    self.uf.bind(rb, wider);
                    Ok(())
                }
            }
            (Type::UInt(a), Type::UInt(b)) => {
                if a == b {
                    Ok(())
                } else {
                    let wider = if a.bits() >= b.bits() { ra } else { rb };
                    self.uf.bind(ra, wider);
                    self.uf.bind(rb, wider);
                    Ok(())
                }
            }
            (Type::Int(a), Type::UInt(b)) | (Type::UInt(b), Type::Int(a)) => {
                let wider = if a.bits() >= b.bits() { ra } else { rb };
                self.uf.bind(ra, wider);
                self.uf.bind(rb, wider);
                Ok(())
            }
            (Type::Float(a), Type::Float(b)) if a == b => Ok(()),
            (Type::Bool, Type::Bool) => Ok(()),
            (Type::Char, Type::Char) => Ok(()),
            (Type::Str, Type::Str) => Ok(()),
            (Type::Unit, Type::Unit) => Ok(()),
            (Type::Never, _) | (_, Type::Never) => Ok(()), // Never is a subtype of everything.

            // References must match mutability and inner type.
            (
                Type::Ref {
                    mutable: m1,
                    inner: i1,
                },
                Type::Ref {
                    mutable: m2,
                    inner: i2,
                },
            ) => {
                if m1 != m2 {
                    return Err(format!(
                        "mutability mismatch: expected {}, found {}",
                        if *m1 { "&mut" } else { "&" },
                        if *m2 { "&mut" } else { "&" }
                    ));
                }
                self.unify(*i1, *i2, store)
            }

            // Pointers.
            (
                Type::Ptr {
                    mutable: m1,
                    inner: i1,
                },
                Type::Ptr {
                    mutable: m2,
                    inner: i2,
                },
            ) => {
                if m1 != m2 {
                    return Err("pointer mutability mismatch".into());
                }
                self.unify(*i1, *i2, store)
            }

            // Optionals.
            (Type::Optional(a), Type::Optional(b)) => self.unify(*a, *b, store),

            // Results.
            (Type::Result { ok: ok1, err: err1 }, Type::Result { ok: ok2, err: err2 }) => {
                self.unify(*ok1, *ok2, store)?;
                self.unify(*err1, *err2, store)
            }

            // Slices.
            (Type::Slice(a), Type::Slice(b)) => self.unify(*a, *b, store),

            // Arrays (size must match).
            (
                Type::Array {
                    element: e1,
                    size: s1,
                },
                Type::Array {
                    element: e2,
                    size: s2,
                },
            ) => {
                if s1 != s2 {
                    return Err(format!(
                        "array size mismatch: expected {}, found {}",
                        s1, s2
                    ));
                }
                self.unify(*e1, *e2, store)
            }

            // Tuples (arity and element types must match).
            (Type::Tuple(a), Type::Tuple(b)) => {
                if a.len() != b.len() {
                    return Err(format!(
                        "tuple arity mismatch: expected {}, found {}",
                        a.len(),
                        b.len()
                    ));
                }
                for (x, y) in a.iter().zip(b.iter()) {
                    self.unify(*x, *y, store)?;
                }
                Ok(())
            }

            // Function types (param count, param types, and return type must match).
            (
                Type::Function {
                    params: p1,
                    ret: r1,
                },
                Type::Function {
                    params: p2,
                    ret: r2,
                },
            ) => {
                if p1.len() != p2.len() {
                    return Err(format!(
                        "function arity mismatch: expected {} params, found {}",
                        p1.len(),
                        p2.len()
                    ));
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

            // Generic type parameters (from a generic struct/fn definition).
            (Type::TypeParam(a), Type::TypeParam(b)) if a == b => Ok(()),

            // Trait objects.
            (Type::DynTrait(a), Type::DynTrait(b)) if a == b => Ok(()),

            // Incompatible types.
            _ => Err(format!(
                "type mismatch: cannot unify {:?} with {:?}",
                ta, tb
            )),
        }
    }

    /// Check if a type variable occurs inside a target type expression to prevent cyclic types.
    pub fn occurs_in(&mut self, var: TypeId, target: TypeId, store: &TypeStore) -> bool {
        let root_target = self.uf.find(target);
        let root_var = self.uf.find(var);
        if root_target == root_var {
            return true;
        }
        match store.get(root_target) {
            Type::Var(_) => false,
            Type::Ref { inner, .. }
            | Type::Ptr { inner, .. }
            | Type::Optional(inner)
            | Type::Slice(inner) => self.occurs_in(root_var, *inner, store),
            Type::Array { element, .. } => self.occurs_in(root_var, *element, store),
            Type::Result { ok, err } => {
                self.occurs_in(root_var, *ok, store) || self.occurs_in(root_var, *err, store)
            }
            Type::Tuple(elems) => elems.iter().any(|&e| self.occurs_in(root_var, e, store)),
            Type::Function { params, ret } => {
                params.iter().any(|&p| self.occurs_in(root_var, p, store))
                    || self.occurs_in(root_var, *ret, store)
            }
            Type::Generic { base, args } => {
                self.occurs_in(root_var, *base, store)
                    || args.iter().any(|&a| self.occurs_in(root_var, a, store))
            }
            _ => false,
        }
    }

    /// Instantiate a generic function/struct signature with fresh type variables.
    pub fn instantiate_generic_signature(
        &mut self,
        params: &[TypeId],
        ret: TypeId,
        generic_names: &[String],
        store: &mut TypeStore,
    ) -> (Vec<TypeId>, TypeId, SubstitutionMap) {
        let mut subst = SubstitutionMap::new();
        for name in generic_names {
            let fresh = store.fresh_var();
            subst.insert(name.clone(), fresh);
        }

        let inst_params = params
            .iter()
            .map(|&p| self.apply_substitution(p, &subst, store))
            .collect();
        let inst_ret = self.apply_substitution(ret, &subst, store);

        (inst_params, inst_ret, subst)
    }

    /// Resolve a TypeId to its final unified type (follows union-find chains).
    pub fn resolve(&mut self, id: TypeId) -> TypeId {
        self.uf.find(id)
    }

    pub fn apply_substitution(
        &mut self,
        ty: TypeId,
        subst: &SubstitutionMap,
        store: &mut TypeStore,
    ) -> TypeId {
        let resolved = self.resolve(ty);
        let ty_val = store.get(resolved).clone();
        match ty_val {
            Type::TypeParam(name) => {
                if let Some(&concrete) = subst.get(&name) {
                    concrete
                } else {
                    resolved
                }
            }
            Type::Ref { mutable, inner } => {
                let inner_sub = self.apply_substitution(inner, subst, store);
                store.insert(Type::Ref {
                    mutable,
                    inner: inner_sub,
                })
            }
            Type::Ptr { mutable, inner } => {
                let inner_sub = self.apply_substitution(inner, subst, store);
                store.insert(Type::Ptr {
                    mutable,
                    inner: inner_sub,
                })
            }
            Type::Optional(inner) => {
                let inner_sub = self.apply_substitution(inner, subst, store);
                store.insert(Type::Optional(inner_sub))
            }
            Type::Result { ok, err } => {
                let ok_sub = self.apply_substitution(ok, subst, store);
                let err_sub = self.apply_substitution(err, subst, store);
                store.insert(Type::Result {
                    ok: ok_sub,
                    err: err_sub,
                })
            }
            Type::Slice(inner) => {
                let inner_sub = self.apply_substitution(inner, subst, store);
                store.insert(Type::Slice(inner_sub))
            }
            Type::Array { element, size } => {
                let elem_sub = self.apply_substitution(element, subst, store);
                store.insert(Type::Array {
                    element: elem_sub,
                    size,
                })
            }
            Type::Tuple(elems) => {
                let elems_sub = elems
                    .iter()
                    .map(|&e| self.apply_substitution(e, subst, store))
                    .collect();
                store.insert(Type::Tuple(elems_sub))
            }
            Type::Function { params, ret } => {
                let params_sub = params
                    .iter()
                    .map(|&p| self.apply_substitution(p, subst, store))
                    .collect();
                let ret_sub = self.apply_substitution(ret, subst, store);
                store.insert(Type::Function {
                    params: params_sub,
                    ret: ret_sub,
                })
            }
            Type::Generic { base, args } => {
                let base_sub = self.apply_substitution(base, subst, store);
                let args_sub = args
                    .iter()
                    .map(|&a| self.apply_substitution(a, subst, store))
                    .collect();
                store.insert(Type::Generic {
                    base: base_sub,
                    args: args_sub,
                })
            }
            _ => resolved,
        }
    }

    /// Recursively resolve all type variables within a compound type.
    pub fn deep_resolve(&mut self, ty: TypeId, store: &mut TypeStore) -> TypeId {
        let root = self.resolve(ty);
        let ty_val = store.get(root).clone();
        match ty_val {
            Type::Var(_) => root,
            Type::Ref { mutable, inner } => {
                let inner_res = self.deep_resolve(inner, store);
                store.insert(Type::Ref {
                    mutable,
                    inner: inner_res,
                })
            }
            Type::Ptr { mutable, inner } => {
                let inner_res = self.deep_resolve(inner, store);
                store.insert(Type::Ptr {
                    mutable,
                    inner: inner_res,
                })
            }
            Type::Optional(inner) => {
                let inner_res = self.deep_resolve(inner, store);
                store.insert(Type::Optional(inner_res))
            }
            Type::Result { ok, err } => {
                let ok_res = self.deep_resolve(ok, store);
                let err_res = self.deep_resolve(err, store);
                store.insert(Type::Result {
                    ok: ok_res,
                    err: err_res,
                })
            }
            Type::Slice(inner) => {
                let inner_res = self.deep_resolve(inner, store);
                store.insert(Type::Slice(inner_res))
            }
            Type::Array { element, size } => {
                let elem_res = self.deep_resolve(element, store);
                store.insert(Type::Array {
                    element: elem_res,
                    size,
                })
            }
            Type::Tuple(elems) => {
                let elems_res = elems.iter().map(|&e| self.deep_resolve(e, store)).collect();
                store.insert(Type::Tuple(elems_res))
            }
            Type::Function { params, ret } => {
                let params_res = params
                    .iter()
                    .map(|&p| self.deep_resolve(p, store))
                    .collect();
                let ret_res = self.deep_resolve(ret, store);
                store.insert(Type::Function {
                    params: params_res,
                    ret: ret_res,
                })
            }
            Type::Generic { base, args } => {
                let base_res = self.deep_resolve(base, store);
                let args_res = args.iter().map(|&a| self.deep_resolve(a, store)).collect();
                store.insert(Type::Generic {
                    base: base_res,
                    args: args_res,
                })
            }
            _ => root,
        }
    }
}

/// A substitution map for generic type parameters.
/// Maps type parameter names to their concrete type instantiations.
pub type SubstitutionMap = std::collections::HashMap<String, TypeId>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_var_unifies_with_concrete() {
        let mut store = TypeStore::new();
        let var = store.fresh_var();
        let int = store.i32();

        let mut engine = InferenceEngine::new(20);
        engine.constrain(var, int, "let binding");
        engine.solve(&store);

        assert_eq!(engine.resolve(var), int);
        assert!(engine.errors.is_empty());
    }

    #[test]
    fn test_two_vars_unify_transitively() {
        let mut store = TypeStore::new();
        let v1 = store.fresh_var();
        let v2 = store.fresh_var();
        let boolean = store.bool();

        let mut engine = InferenceEngine::new(20);
        engine.constrain(v1, v2, "v1 = v2");
        engine.constrain(v2, boolean, "v2 = bool");
        engine.solve(&store);

        assert_eq!(engine.resolve(v1), boolean);
        assert_eq!(engine.resolve(v2), boolean);
    }

    #[test]
    fn test_concrete_type_mismatch() {
        let store = TypeStore::new();
        let int = store.i32();
        let boolean = store.bool();

        let mut engine = InferenceEngine::new(20);
        engine.constrain(int, boolean, "assigning bool to int");
        engine.solve(&store);

        assert_eq!(engine.errors.len(), 1);
        assert!(engine.errors[0].message.contains("type mismatch"));
    }

    #[test]
    fn test_any_unifies_with_everything() {
        let store = TypeStore::new();
        let any = store.any();
        let int = store.i32();

        let mut engine = InferenceEngine::new(20);
        engine.constrain(any, int, "any = i32");
        engine.solve(&store);

        assert!(engine.errors.is_empty());
    }

    #[test]
    fn test_function_type_unification() {
        let mut store = TypeStore::new();
        let v = store.fresh_var();
        let int = store.i32();
        let boolean = store.bool();

        let fn1 = store.insert(Type::Function {
            params: vec![v],
            ret: boolean,
        });
        let fn2 = store.insert(Type::Function {
            params: vec![int],
            ret: boolean,
        });

        let mut engine = InferenceEngine::new(20);
        engine.constrain(fn1, fn2, "fn call");
        engine.solve(&store);

        assert!(engine.errors.is_empty());
        assert_eq!(engine.resolve(v), int);
    }

    #[test]
    fn test_function_arity_mismatch() {
        let mut store = TypeStore::new();
        let int = store.i32();
        let boolean = store.bool();

        let fn1 = store.insert(Type::Function {
            params: vec![int],
            ret: boolean,
        });
        let fn2 = store.insert(Type::Function {
            params: vec![int, int],
            ret: boolean,
        });

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

        let r1 = store.insert(Type::Ref {
            mutable: false,
            inner: int,
        });
        let r2 = store.insert(Type::Ref {
            mutable: true,
            inner: int,
        });

        let mut engine = InferenceEngine::new(20);
        engine.constrain(r1, r2, "ref mismatch");
        engine.solve(&store);

        assert_eq!(engine.errors.len(), 1);
        assert!(engine.errors[0].message.contains("mutability mismatch"));
    }

    #[test]
    fn test_occurs_check_prevents_cyclic_types() {
        let mut store = TypeStore::new();
        let var_t = store.fresh_var();

        // Try to unify T with (T, i32) -> cyclic type T = (T, i32)
        let int = store.i32();
        let tuple_with_t = store.insert(Type::Tuple(vec![var_t, int]));

        let mut engine = InferenceEngine::new(20);
        engine.constrain(var_t, tuple_with_t, "cyclic assignment");
        engine.solve(&store);

        assert_eq!(engine.errors.len(), 1);
        assert!(engine.errors[0].message.contains("occurs check failed"));
    }

    #[test]
    fn test_instantiate_generic_signature() {
        let mut store = TypeStore::new();
        let param_t = store.insert(Type::TypeParam("T".into()));
        let int = store.i32();

        // fn map(val: T) -> (T, i32)
        let ret_tuple = store.insert(Type::Tuple(vec![param_t, int]));

        let mut engine = InferenceEngine::new(20);
        let (inst_params, inst_ret, subst) =
            engine.instantiate_generic_signature(&[param_t], ret_tuple, &["T".into()], &mut store);

        assert_eq!(inst_params.len(), 1);
        assert_ne!(inst_params[0], param_t); // Fresh variable instantiated
        assert!(subst.contains_key("T"));

        // Unify instantiated parameter with bool
        let bool_ty = store.bool();
        engine.constrain(inst_params[0], bool_ty, "arg passing");
        engine.solve(&store);

        assert!(engine.errors.is_empty());
        let resolved_ret = engine.deep_resolve(inst_ret, &mut store);
        let resolved_val = store.get(resolved_ret).clone();
        if let Type::Tuple(elems) = resolved_val {
            assert_eq!(elems[0], bool_ty);
            assert_eq!(elems[1], int);
        } else {
            panic!("Expected tuple type");
        }
    }
}
