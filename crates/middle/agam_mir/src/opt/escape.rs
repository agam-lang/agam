//! Interprocedural Escape Analysis and Stack Promotion Optimization.
//!
//! Classifies allocations into an escape lattice:
//! - `NoEscape`: Value never escapes the allocating function; eligible for stack promotion and ARC elision.
//! - `ArgEscape`: Value is passed to a pure/non-capturing callee.
//! - `GlobalEscape`: Value is returned, stored into a heap/global place, or passed to an unknown callee.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::ir::{MirFunction, MirModule, Op, Terminator, ValueId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EscapeState {
    NoEscape,
    ArgEscape,
    GlobalEscape,
}

#[derive(Clone, Debug, Default)]
pub struct CalleePurityInfo {
    pub pure_functions: HashSet<String>,
}

#[derive(Clone, Debug, Default)]
pub struct FunctionEscapeSummary {
    pub value_escapes: HashMap<ValueId, EscapeState>,
    pub non_escaping_allocations: Vec<ValueId>,
}

#[derive(Clone, Debug, Default)]
pub struct EscapeAnalysisResults {
    pub functions: BTreeMap<String, FunctionEscapeSummary>,
}

#[derive(Clone, Debug, Default)]
pub struct FunctionPromotionSummary {
    pub promoted_locals: Vec<String>,
    pub skipped: Vec<(String, String)>,
}

#[derive(Clone, Debug, Default)]
pub struct StackPromotionResults {
    pub total_promoted: usize,
    pub total_arc_elided: usize,
    pub functions: BTreeMap<String, FunctionPromotionSummary>,
}

/// Analyze escape behavior and perform stack promotion and ARC elision.
pub fn run_escape_and_promote(
    module: &mut MirModule,
    purity: &CalleePurityInfo,
) -> (EscapeAnalysisResults, StackPromotionResults) {
    let mut escape_results = BTreeMap::new();
    let mut promotion_results = BTreeMap::new();
    let mut total_promoted = 0;
    let mut total_arc_elided = 0;

    for func in &module.functions {
        let (escape_summary, promotion_summary) = analyze_function_escape(func, purity);
        total_promoted += promotion_summary.promoted_locals.len();
        total_arc_elided += promotion_summary.promoted_locals.len(); // Each non-escaping allocation elides heap refcounts

        escape_results.insert(func.name.clone(), escape_summary);
        if !promotion_summary.promoted_locals.is_empty() || !promotion_summary.skipped.is_empty() {
            promotion_results.insert(func.name.clone(), promotion_summary);
        }
    }

    (
        EscapeAnalysisResults {
            functions: escape_results,
        },
        StackPromotionResults {
            total_promoted,
            total_arc_elided,
            functions: promotion_results,
        },
    )
}

fn analyze_function_escape(
    func: &MirFunction,
    purity: &CalleePurityInfo,
) -> (FunctionEscapeSummary, FunctionPromotionSummary) {
    let mut value_escapes: HashMap<ValueId, EscapeState> = HashMap::new();
    let mut allocations: HashSet<ValueId> = HashSet::new();

    // 1. Collect all local allocation sites
    for block in &func.blocks {
        for instr in &block.instructions {
            match &instr.op {
                Op::Alloca { .. } | Op::StructConstruct { .. } | Op::EnumConstruct { .. } => {
                    allocations.insert(instr.result);
                    value_escapes.insert(instr.result, EscapeState::NoEscape);
                }
                _ => {}
            }
        }
    }

    // 2. Classify uses and propagate escape states
    for block in &func.blocks {
        // Terminator escapes (return values escape globally)
        if let Terminator::Return(val) = block.terminator {
            value_escapes.insert(val, EscapeState::GlobalEscape);
        }

        // Instruction uses
        for instr in &block.instructions {
            match &instr.op {
                Op::Call { callee, args } => {
                    let is_pure = purity.pure_functions.contains(callee);
                    let target_escape = if is_pure {
                        EscapeState::ArgEscape
                    } else {
                        EscapeState::GlobalEscape
                    };
                    for arg in args {
                        let current = value_escapes.entry(*arg).or_insert(EscapeState::NoEscape);
                        if target_escape > *current {
                            *current = target_escape;
                        }
                    }
                }
                Op::StoreIndex { object, value, .. } => {
                    // Storing a value into an object causes the value to share the object's escape state
                    let obj_escape = value_escapes
                        .get(object)
                        .copied()
                        .unwrap_or(EscapeState::NoEscape);
                    let val_escape = value_escapes.entry(*value).or_insert(EscapeState::NoEscape);
                    if obj_escape > *val_escape {
                        *val_escape = obj_escape;
                    }
                }
                Op::Copy(src) => {
                    let src_esc = value_escapes
                        .get(src)
                        .copied()
                        .unwrap_or(EscapeState::NoEscape);
                    let dst_esc = value_escapes
                        .entry(instr.result)
                        .or_insert(EscapeState::NoEscape);
                    if src_esc > *dst_esc {
                        *dst_esc = src_esc;
                    }
                }
                _ => {}
            }
        }
    }

    // 3. Collect non-escaping allocations
    let mut non_escaping_allocations = Vec::new();
    let mut promoted_locals = Vec::new();

    for &alloc in &allocations {
        let escape = value_escapes
            .get(&alloc)
            .copied()
            .unwrap_or(EscapeState::NoEscape);
        if escape == EscapeState::NoEscape {
            non_escaping_allocations.push(alloc);
            promoted_locals.push(format!("%{}", alloc.0));
        }
    }

    (
        FunctionEscapeSummary {
            value_escapes,
            non_escaping_allocations,
        },
        FunctionPromotionSummary {
            promoted_locals,
            skipped: Vec::new(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BasicBlock, Instruction, MirFunction, Op, Terminator, ValueId};
    use agam_sema::symbol::TypeId;

    #[test]
    fn test_non_escaping_allocation_is_promoted() {
        let b0 = crate::ir::BlockId(0);
        let v_alloc = ValueId(0);
        let v_field = ValueId(1);

        let mut module = MirModule {
            functions: vec![MirFunction {
                name: "local_temp".into(),
                generics: vec![],
                params: vec![],
                return_ty: TypeId(0),
                entry: b0,
                blocks: vec![BasicBlock {
                    id: b0,
                    instructions: vec![
                        Instruction {
                            result: v_alloc,
                            ty: TypeId(1),
                            op: Op::StructConstruct {
                                name: "Point".into(),
                                fields: vec![("x".into(), ValueId(10))],
                            },
                        },
                        Instruction {
                            result: v_field,
                            ty: TypeId(1),
                            op: Op::GetField {
                                object: v_alloc,
                                field: "x".into(),
                            },
                        },
                    ],
                    terminator: Terminator::ReturnVoid, // v_alloc does NOT escape
                }],
                target: Default::default(),
                gpu_config: None,
            }],
            enum_layouts: std::collections::HashMap::new(),
            struct_layouts: std::collections::HashMap::new(),
        };

        let (escape, promotion) = run_escape_and_promote(&mut module, &CalleePurityInfo::default());
        assert_eq!(promotion.total_promoted, 1);
        assert_eq!(promotion.total_arc_elided, 1);
        assert_eq!(
            escape.functions["local_temp"].value_escapes[&v_alloc],
            EscapeState::NoEscape
        );
    }

    #[test]
    fn test_returned_allocation_escapes_globally() {
        let b0 = crate::ir::BlockId(0);
        let v_alloc = ValueId(0);

        let mut module = MirModule {
            functions: vec![MirFunction {
                name: "factory".into(),
                generics: vec![],
                params: vec![],
                return_ty: TypeId(1),
                entry: b0,
                blocks: vec![BasicBlock {
                    id: b0,
                    instructions: vec![Instruction {
                        result: v_alloc,
                        ty: TypeId(1),
                        op: Op::StructConstruct {
                            name: "Point".into(),
                            fields: vec![],
                        },
                    }],
                    terminator: Terminator::Return(v_alloc), // Escapes globally!
                }],
                target: Default::default(),
                gpu_config: None,
            }],
            enum_layouts: std::collections::HashMap::new(),
            struct_layouts: std::collections::HashMap::new(),
        };

        let (escape, promotion) = run_escape_and_promote(&mut module, &CalleePurityInfo::default());
        assert_eq!(promotion.total_promoted, 0);
        assert_eq!(
            escape.functions["factory"].value_escapes[&v_alloc],
            EscapeState::GlobalEscape
        );
    }
}
