//! Monomorphization pass — generic function specialization.
//!
//! Walks the MIR module and discovers all concrete instantiations of
//! generic functions. For each unique type substitution, clones the
//! generic function's MIR, replaces `TypeParam` references with
//! concrete types, and generates uniquely mangled function names.
//!
//! ## Design (F6: Sandhi — Type Junction Rules)
//!
//! Monomorphization follows the *sandhi* principle from the Agam design
//! philosophy: when types combine at function call sites, the resulting
//! specialization follows predictable, documented rules rather than
//! ad-hoc coercions.

use std::collections::{HashMap, HashSet};

use agam_sema::symbol::TypeId;
use agam_sema::types::{Type, TypeStore};

use crate::ir::{BasicBlock, Instruction, MirFunction, MirModule, Op};

/// A concrete instantiation key: function name + type arguments.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MonomorphKey {
    /// The original generic function name.
    pub base_name: String,
    /// The concrete TypeIds substituted for each generic parameter.
    pub type_args: Vec<TypeId>,
}

impl MonomorphKey {
    /// Generate a mangled name for this specialization.
    ///
    /// Format: `base_name__type1__type2__...`
    pub fn mangled_name(&self, types: &TypeStore) -> String {
        if self.type_args.is_empty() {
            return self.base_name.clone();
        }
        let mut name = self.base_name.clone();
        for &arg in &self.type_args {
            name.push_str("__");
            name.push_str(&type_name_for_mangling(types, arg));
        }
        name
    }
}

/// Produce a short name for a type suitable for function name mangling.
fn type_name_for_mangling(types: &TypeStore, ty: TypeId) -> String {
    match types.get(ty) {
        Type::Int(sz) => format!("{:?}", sz).to_lowercase(),
        Type::UInt(sz) => format!("u{:?}", sz).to_lowercase(),
        Type::Float(sz) => format!("{:?}", sz).to_lowercase(),
        Type::Bool => "bool".into(),
        Type::Char => "char".into(),
        Type::Str => "str".into(),
        Type::Unit => "unit".into(),
        Type::Never => "never".into(),
        Type::Any => "any".into(),
        Type::Named(sym) => format!("t{}", sym.0),
        Type::TypeParam(name) => name.clone(),
        Type::Generic { base, args } => {
            let base_name = type_name_for_mangling(types, *base);
            let arg_names: Vec<String> = args
                .iter()
                .map(|a| type_name_for_mangling(types, *a))
                .collect();
            format!("{}__{}", base_name, arg_names.join("_"))
        }
        _ => format!("t{}", ty.0),
    }
}

/// Result of monomorphization: the specialized functions to add.
pub struct MonomorphResult {
    /// New specialized functions to append to the module.
    pub specialized: Vec<MirFunction>,
    /// Mapping from (call_site_callee, type_args) to mangled name.
    pub renames: HashMap<MonomorphKey, String>,
}

/// Substitute all `TypeParam` occurrences in a TypeId according to the
/// given mapping. Returns the substituted TypeId, inserting new compound
/// types into the store as needed.
fn substitute_type(
    ty: TypeId,
    subst: &HashMap<String, TypeId>,
    types: &mut TypeStore,
) -> TypeId {
    match types.get(ty).clone() {
        Type::TypeParam(name) => {
            if let Some(&concrete) = subst.get(&name) {
                concrete
            } else {
                ty
            }
        }
        Type::Array { element, size } => {
            let new_elem = substitute_type(element, subst, types);
            if new_elem == element {
                ty
            } else {
                types.insert(Type::Array { element: new_elem, size })
            }
        }
        Type::Slice(inner) => {
            let new_inner = substitute_type(inner, subst, types);
            if new_inner == inner {
                ty
            } else {
                types.insert(Type::Slice(new_inner))
            }
        }
        Type::Tuple(elems) => {
            let new_elems: Vec<TypeId> = elems.iter().map(|&e| substitute_type(e, subst, types)).collect();
            if new_elems == elems {
                ty
            } else {
                types.insert(Type::Tuple(new_elems))
            }
        }
        Type::Ref { mutable, inner } => {
            let new_inner = substitute_type(inner, subst, types);
            if new_inner == inner {
                ty
            } else {
                types.insert(Type::Ref { mutable, inner: new_inner })
            }
        }
        Type::Ptr { mutable, inner } => {
            let new_inner = substitute_type(inner, subst, types);
            if new_inner == inner {
                ty
            } else {
                types.insert(Type::Ptr { mutable, inner: new_inner })
            }
        }
        Type::Optional(inner) => {
            let new_inner = substitute_type(inner, subst, types);
            if new_inner == inner {
                ty
            } else {
                types.insert(Type::Optional(new_inner))
            }
        }
        Type::Result { ok, err } => {
            let new_ok = substitute_type(ok, subst, types);
            let new_err = substitute_type(err, subst, types);
            if new_ok == ok && new_err == err {
                ty
            } else {
                types.insert(Type::Result { ok: new_ok, err: new_err })
            }
        }
        Type::Generic { base, args } => {
            let new_base = substitute_type(base, subst, types);
            let new_args: Vec<TypeId> = args.iter().map(|&a| substitute_type(a, subst, types)).collect();
            if new_base == base && new_args == args {
                ty
            } else {
                types.insert(Type::Generic { base: new_base, args: new_args })
            }
        }
        Type::Function { params, ret } => {
            let new_params: Vec<TypeId> = params.iter().map(|&p| substitute_type(p, subst, types)).collect();
            let new_ret = substitute_type(ret, subst, types);
            if new_params == params && new_ret == ret {
                ty
            } else {
                types.insert(Type::Function { params: new_params, ret: new_ret })
            }
        }
        // Primitives, Named, Var, Any, Error, etc. — no type params inside.
        _ => ty,
    }
}

/// Substitute all TypeParam references throughout an entire MirFunction body.
fn substitute_function(
    func: &MirFunction,
    subst: &HashMap<String, TypeId>,
    mangled_name: &str,
    types: &mut TypeStore,
) -> MirFunction {
    let new_params = func
        .params
        .iter()
        .map(|p| crate::ir::MirParam {
            name: p.name.clone(),
            value: p.value,
            ty: substitute_type(p.ty, subst, types),
            gpu_abi: p.gpu_abi,
            memory_type: p.memory_type,
        })
        .collect();

    let new_return_ty = substitute_type(func.return_ty, subst, types);

    let new_blocks = func
        .blocks
        .iter()
        .map(|block| substitute_block(block, subst, types))
        .collect();

    MirFunction {
        name: mangled_name.to_string(),
        generics: vec![], // Specialized — no longer generic.
        params: new_params,
        return_ty: new_return_ty,
        blocks: new_blocks,
        entry: func.entry,
        target: func.target,
        gpu_config: func.gpu_config.clone(),
    }
}

/// Substitute types throughout a basic block.
fn substitute_block(
    block: &BasicBlock,
    subst: &HashMap<String, TypeId>,
    types: &mut TypeStore,
) -> BasicBlock {
    BasicBlock {
        id: block.id,
        instructions: block
            .instructions
            .iter()
            .map(|instr| Instruction {
                result: instr.result,
                ty: substitute_type(instr.ty, subst, types),
                op: instr.op.clone(),
            })
            .collect(),
        terminator: block.terminator.clone(),
    }
}

/// Run monomorphization on a MIR module.
///
/// Identifies generic functions, discovers all concrete instantiations
/// at call sites, clones the generic MIR with substituted types, and
/// generates mangled function names for each specialization.
pub fn monomorphize(module: &MirModule, types: &mut TypeStore) -> MonomorphResult {
    let mut specialized = Vec::new();
    let mut renames = HashMap::new();

    // Index generic functions by name for fast lookup.
    let generic_fns: HashMap<String, &MirFunction> = module
        .functions
        .iter()
        .filter(|f| !f.generics.is_empty())
        .map(|f| (f.name.clone(), f))
        .collect();

    if generic_fns.is_empty() {
        return MonomorphResult { specialized, renames };
    }

    // Also identify generic functions by TypeParam in params (backward compat).
    let type_param_fns: HashSet<String> = module
        .functions
        .iter()
        .filter(|f| {
            f.params
                .iter()
                .any(|p| matches!(types.get(p.ty), Type::TypeParam(_)))
        })
        .map(|f| f.name.clone())
        .collect();

    let all_generic_names: HashSet<&str> = generic_fns
        .keys()
        .map(|s| s.as_str())
        .chain(type_param_fns.iter().map(|s| s.as_str()))
        .collect();

    // Scan all call sites in all functions to find concrete instantiations.
    for func in &module.functions {
        for block in &func.blocks {
            for instr in &block.instructions {
                if let Op::Call { callee, args } = &instr.op {
                    if !all_generic_names.contains(callee.as_str()) {
                        continue;
                    }

                    // Collect the concrete types of the arguments.
                    let arg_types: Vec<TypeId> = args
                        .iter()
                        .filter_map(|arg_val| {
                            // Find the instruction that produced this value
                            // and use its result type.
                            func.blocks
                                .iter()
                                .flat_map(|b| &b.instructions)
                                .find(|i| i.result == *arg_val)
                                .map(|i| i.ty)
                        })
                        .collect();

                    let key = MonomorphKey {
                        base_name: callee.clone(),
                        type_args: arg_types.clone(),
                    };

                    if renames.contains_key(&key) {
                        continue;
                    }

                    let mangled = key.mangled_name(types);
                    renames.insert(key, mangled.clone());

                    // Clone the generic function's MIR with substituted types.
                    if let Some(generic_func) = generic_fns.get(callee) {
                        // Build substitution map: generic param name -> concrete type.
                        let subst: HashMap<String, TypeId> = generic_func
                            .generics
                            .iter()
                            .zip(arg_types.iter())
                            .map(|(name, &ty)| (name.clone(), ty))
                            .collect();

                        let spec_func =
                            substitute_function(generic_func, &subst, &mangled, types);
                        specialized.push(spec_func);
                    }
                }
            }
        }
    }

    MonomorphResult {
        specialized,
        renames,
    }
}

/// Apply monomorphization results to the module: append specialized
/// functions and rewrite call sites to use mangled names.
pub fn apply_monomorphization(module: &mut MirModule, result: MonomorphResult) {
    if result.renames.is_empty() {
        return;
    }

    let rename_map: HashMap<MonomorphKey, String> = result.renames;

    // Rewrite call sites in existing functions.
    for func in &mut module.functions {
        // First pass: collect which (block, instr) pairs need renaming.
        let mut rewrites: Vec<(usize, usize, String)> = Vec::new();
        for (bi, block) in func.blocks.iter().enumerate() {
            for (ii, instr) in block.instructions.iter().enumerate() {
                if let Op::Call { callee, args } = &instr.op {
                    let arg_types: Vec<TypeId> = args
                        .iter()
                        .filter_map(|arg_val| {
                            func.blocks
                                .iter()
                                .flat_map(|b| &b.instructions)
                                .find(|i| i.result == *arg_val)
                                .map(|i| i.ty)
                        })
                        .collect();

                    let key = MonomorphKey {
                        base_name: callee.clone(),
                        type_args: arg_types,
                    };

                    if let Some(mangled) = rename_map.get(&key) {
                        rewrites.push((bi, ii, mangled.clone()));
                    }
                }
            }
        }

        // Second pass: apply rewrites.
        for (bi, ii, mangled) in rewrites {
            if let Op::Call { callee, .. } = &mut func.blocks[bi].instructions[ii].op {
                *callee = mangled;
            }
        }
    }

    // Remove the original generic functions (they're replaced by specializations).
    module
        .functions
        .retain(|f| f.generics.is_empty());

    // Append specialized functions.
    module.functions.extend(result.specialized);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::*;

    #[test]
    fn test_monomorph_key_mangled_name_no_args() {
        let types = TypeStore::new();
        let key = MonomorphKey {
            base_name: "identity".into(),
            type_args: vec![],
        };
        assert_eq!(key.mangled_name(&types), "identity");
    }

    #[test]
    fn test_monomorph_key_mangled_name_with_args() {
        let types = TypeStore::new();
        let key = MonomorphKey {
            base_name: "map".into(),
            type_args: vec![types.i32(), types.str()],
        };
        let mangled = key.mangled_name(&types);
        assert!(mangled.starts_with("map__"));
        assert!(mangled.contains("i32"));
    }

    #[test]
    fn test_empty_module_monomorphize() {
        let mut types = TypeStore::new();
        let module = MirModule {
            functions: vec![],
            enum_layouts: std::collections::HashMap::new(),
            struct_layouts: std::collections::HashMap::new(),
        };
        let result = monomorphize(&module, &mut types);
        assert!(result.specialized.is_empty());
        assert!(result.renames.is_empty());
    }

    #[test]
    fn test_substitute_type_replaces_type_param() {
        let mut types = TypeStore::new();
        let t_param = types.insert(Type::TypeParam("T".into()));
        let mut subst = HashMap::new();
        subst.insert("T".into(), types.i32());

        let result = substitute_type(t_param, &subst, &mut types);
        assert_eq!(result, types.i32());
    }

    #[test]
    fn test_substitute_type_leaves_concrete_unchanged() {
        let mut types = TypeStore::new();
        let i32_ty = types.i32();
        let subst = HashMap::new();

        let result = substitute_type(i32_ty, &subst, &mut types);
        assert_eq!(result, i32_ty);
    }

    #[test]
    fn test_substitute_type_recurses_into_array() {
        let mut types = TypeStore::new();
        let t_param = types.insert(Type::TypeParam("T".into()));
        let arr_ty = types.insert(Type::Array { element: t_param, size: 10 });
        let mut subst = HashMap::new();
        subst.insert("T".into(), types.f64());

        let result = substitute_type(arr_ty, &subst, &mut types);
        match types.get(result) {
            Type::Array { element, size: 10 } => {
                assert_eq!(*element, types.f64());
            }
            other => panic!("expected Array, got {:?}", other),
        }
    }

    #[test]
    fn test_monomorphize_creates_specialization() {
        let mut types = TypeStore::new();
        let t_param = types.insert(Type::TypeParam("T".into()));

        // Generic function: fn identity<T>(x: T) -> T
        let generic_fn = MirFunction {
            name: "identity".into(),
            generics: vec!["T".into()],
            params: vec![MirParam {
                name: "x".into(),
                value: ValueId(0),
                ty: t_param,
                gpu_abi: Default::default(),
                memory_type: None,
            }],
            return_ty: t_param,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                instructions: vec![Instruction {
                    result: ValueId(1),
                    ty: t_param,
                    op: Op::Copy(ValueId(0)),
                }],
                terminator: Terminator::Return(ValueId(1)),
            }],
            entry: BlockId(0),
            target: Default::default(),
            gpu_config: None,
        };

        // Caller: fn main() -> i32 { return identity(42) }
        let caller_fn = MirFunction {
            name: "main".into(),
            generics: vec![],
            params: vec![],
            return_ty: types.i32(),
            blocks: vec![BasicBlock {
                id: BlockId(0),
                instructions: vec![
                    Instruction {
                        result: ValueId(0),
                        ty: types.i32(),
                        op: Op::ConstInt(42),
                    },
                    Instruction {
                        result: ValueId(1),
                        ty: types.i32(),
                        op: Op::Call {
                            callee: "identity".into(),
                            args: vec![ValueId(0)],
                        },
                    },
                ],
                terminator: Terminator::Return(ValueId(1)),
            }],
            entry: BlockId(0),
            target: Default::default(),
            gpu_config: None,
        };

        let module = MirModule {
            functions: vec![generic_fn, caller_fn],
            enum_layouts: std::collections::HashMap::new(),
            struct_layouts: std::collections::HashMap::new(),
        };

        let result = monomorphize(&module, &mut types);

        // Should produce one specialization.
        assert_eq!(result.specialized.len(), 1);
        assert_eq!(result.renames.len(), 1);

        // The specialized function should have i32 types, not TypeParam.
        let spec = &result.specialized[0];
        assert!(spec.name.contains("identity__"));
        assert!(spec.generics.is_empty());
        assert_eq!(spec.params[0].ty, types.i32());
        assert_eq!(spec.return_ty, types.i32());

        // The instruction inside should also have substituted type.
        assert_eq!(spec.blocks[0].instructions[0].ty, types.i32());
    }

    #[test]
    fn test_apply_monomorphization_rewrites_calls_and_removes_generic() {
        let mut types = TypeStore::new();
        let t_param = types.insert(Type::TypeParam("T".into()));

        let generic_fn = MirFunction {
            name: "identity".into(),
            generics: vec!["T".into()],
            params: vec![MirParam {
                name: "x".into(),
                value: ValueId(0),
                ty: t_param,
                gpu_abi: Default::default(),
                memory_type: None,
            }],
            return_ty: t_param,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                instructions: vec![Instruction {
                    result: ValueId(1),
                    ty: t_param,
                    op: Op::Copy(ValueId(0)),
                }],
                terminator: Terminator::Return(ValueId(1)),
            }],
            entry: BlockId(0),
            target: Default::default(),
            gpu_config: None,
        };

        let caller_fn = MirFunction {
            name: "main".into(),
            generics: vec![],
            params: vec![],
            return_ty: types.i32(),
            blocks: vec![BasicBlock {
                id: BlockId(0),
                instructions: vec![
                    Instruction {
                        result: ValueId(0),
                        ty: types.i32(),
                        op: Op::ConstInt(42),
                    },
                    Instruction {
                        result: ValueId(1),
                        ty: types.i32(),
                        op: Op::Call {
                            callee: "identity".into(),
                            args: vec![ValueId(0)],
                        },
                    },
                ],
                terminator: Terminator::Return(ValueId(1)),
            }],
            entry: BlockId(0),
            target: Default::default(),
            gpu_config: None,
        };

        let mut module = MirModule {
            functions: vec![generic_fn, caller_fn],
            enum_layouts: std::collections::HashMap::new(),
            struct_layouts: std::collections::HashMap::new(),
        };

        let result = monomorphize(&module, &mut types);
        apply_monomorphization(&mut module, result);

        // Generic function should be removed, specialization added.
        assert!(!module.functions.iter().any(|f| f.name == "identity"));
        assert!(module.functions.iter().any(|f| f.name.contains("identity__")));

        // Call in main should be rewritten.
        let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
        if let Op::Call { callee, .. } = &main_fn.blocks[0].instructions[1].op {
            assert!(callee.contains("identity__"), "call not rewritten: {}", callee);
        } else {
            panic!("expected Call op");
        }
    }
}
