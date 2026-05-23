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

use crate::ir::{MirFunction, MirModule, Op};

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
            let arg_names: Vec<String> = args.iter().map(|a| type_name_for_mangling(types, *a)).collect();
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

/// Run monomorphization on a MIR module.
///
/// Currently a structural pass that identifies generic functions (those
/// containing `TypeParam` references) and prepares the infrastructure
/// for specialization. Full substitution of type parameters within
/// function bodies will be implemented when the HIR lowering emits
/// generic type metadata on MIR functions.
pub fn monomorphize(module: &MirModule, types: &TypeStore) -> MonomorphResult {
    let specialized = Vec::new();
    let mut renames = HashMap::new();

    // Identify generic functions (those whose params reference TypeParam types).
    let generic_fns: HashSet<String> = module
        .functions
        .iter()
        .filter(|f| {
            f.params
                .iter()
                .any(|p| matches!(types.get(p.ty), Type::TypeParam(_)))
        })
        .map(|f| f.name.clone())
        .collect();

    // Scan all call sites in all functions to find concrete instantiations.
    for func in &module.functions {
        for block in &func.blocks {
            for instr in &block.instructions {
                if let Op::Call { callee, args } = &instr.op {
                    if generic_fns.contains(callee) {
                        // Collect the concrete types of the arguments.
                        let arg_types: Vec<TypeId> = args
                            .iter()
                            .filter_map(|arg_val| {
                                // Find the instruction that produced this value
                                // and use its result type.
                                func.blocks.iter().flat_map(|b| &b.instructions).find(|i| i.result == *arg_val).map(|i| i.ty)
                            })
                            .collect();

                        let key = MonomorphKey {
                            base_name: callee.clone(),
                            type_args: arg_types,
                        };

                        if !renames.contains_key(&key) {
                            let mangled = key.mangled_name(types);
                            renames.insert(key, mangled);
                            // TODO: Clone the generic function's MIR with
                            // substituted types once HIR emits generic metadata.
                        }
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let types = TypeStore::new();
        let module = MirModule {
            functions: vec![],
        };
        let result = monomorphize(&module, &types);
        assert!(result.specialized.is_empty());
        assert!(result.renames.is_empty());
    }
}
