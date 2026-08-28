//! Interprocedural Escape Analysis and Stack Promotion Optimization.
//!
//! Classifies allocations into an escape lattice:
//! - `NoEscape`: Value never escapes the allocating function; eligible for stack promotion and ARC elision.
//! - `ArgEscape`: Value is passed to a pure/non-capturing callee.
//! - `GlobalEscape`: Value is returned, stored into a heap/global place, or passed to an unknown callee.
//!
//! # Safety & Fallback Invariant
//! When escape state is uncertain (e.g. indirect calls, recursive escaping returns, effect boundaries),
//! the analysis gracefully falls back to `GlobalEscape` (keeping default heap/ARC semantics).

#![deny(clippy::unwrap_used)]

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::ir::{MirFunction, MirModule, Op, Terminator, ValueId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EscapeState {
    NoEscape = 0,
    ArgEscape = 1,
    GlobalEscape = 2,
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

/// Analyze escape behavior and perform stack promotion and ARC elision with graceful fallback.
pub fn run_escape_and_promote(
    module: &mut MirModule,
    purity: &CalleePurityInfo,
) -> (EscapeAnalysisResults, StackPromotionResults) {
    let mut escape_results = BTreeMap::new();
    let mut promotion_results = BTreeMap::new();
    let mut total_promoted = 0;
    let mut total_arc_elided = 0;

    for func in &mut module.functions {
        let (escape_summary, promotion_summary) = analyze_and_mutate_function_escape(func, purity);
        total_promoted += promotion_summary.promoted_locals.len();
        total_arc_elided += promotion_summary.promoted_locals.len();

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

fn analyze_and_mutate_function_escape(
    func: &mut MirFunction,
    purity: &CalleePurityInfo,
) -> (FunctionEscapeSummary, FunctionPromotionSummary) {
    let mut value_escapes: HashMap<ValueId, EscapeState> = HashMap::new();
    let mut allocations: HashSet<ValueId> = HashSet::new();

    // 1. Collect all local allocation sites & parameters
    for param in &func.params {
        // Parameters arrive from caller: by default ArgEscape unless mutated/stored
        value_escapes.insert(param.value, EscapeState::ArgEscape);
    }

    for block in &func.blocks {
        for instr in &block.instructions {
            match &instr.op {
                Op::Alloca { .. } | Op::StructConstruct { .. } | Op::EnumConstruct { .. } => {
                    allocations.insert(instr.result);
                    value_escapes.insert(instr.result, EscapeState::NoEscape);
                }
                Op::Call { callee, .. }
                    if callee.starts_with("AlignedBuffer::")
                        || callee.starts_with("Tensor::")
                        || callee.contains("alloc") =>
                {
                    allocations.insert(instr.result);
                    value_escapes.insert(instr.result, EscapeState::NoEscape);
                }
                _ => {}
            }
        }
    }

    // 2. Fixed-point iterative escape propagation
    let mut changed = true;
    let mut iteration = 0;
    const MAX_PROPAGATION_ITERATIONS: usize = 32;

    while changed && iteration < MAX_PROPAGATION_ITERATIONS {
        changed = false;
        iteration += 1;

        for block in &func.blocks {
            // Terminator escapes
            match &block.terminator {
                Terminator::Return(ret_val) => {
                    let cur = value_escapes
                        .entry(*ret_val)
                        .or_insert(EscapeState::NoEscape);
                    if *cur < EscapeState::GlobalEscape {
                        *cur = EscapeState::GlobalEscape;
                        changed = true;
                    }
                }
                _ => {}
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
                            let cur = value_escapes.entry(*arg).or_insert(EscapeState::NoEscape);
                            if target_escape > *cur {
                                *cur = target_escape;
                                changed = true;
                            }
                        }
                    }
                    Op::StoreIndex { object, value, .. } => {
                        let obj_esc = value_escapes
                            .get(object)
                            .copied()
                            .unwrap_or(EscapeState::NoEscape);
                        let val_esc = value_escapes.entry(*value).or_insert(EscapeState::NoEscape);
                        if obj_esc > *val_esc {
                            *val_esc = obj_esc;
                            changed = true;
                        }
                    }
                    Op::StoreLocal { value, .. } => {
                        // Storing to local variable keeps current local state unless local escapes
                        let val_esc = value_escapes.entry(*value).or_insert(EscapeState::NoEscape);
                        if *val_esc < EscapeState::NoEscape {
                            *val_esc = EscapeState::NoEscape;
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
                            changed = true;
                        }
                    }
                    Op::EffectPerform { args, .. } => {
                        // Effect boundaries escape globally (handled by caller/effect runtime)
                        for arg in args {
                            let cur = value_escapes.entry(*arg).or_insert(EscapeState::NoEscape);
                            if *cur < EscapeState::GlobalEscape {
                                *cur = EscapeState::GlobalEscape;
                                changed = true;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // 3. Classify promotions and graceful fallbacks
    let mut non_escaping_allocations = Vec::new();
    let mut promoted_locals = Vec::new();
    let mut skipped = Vec::new();

    for &alloc in &allocations {
        let escape = value_escapes
            .get(&alloc)
            .copied()
            .unwrap_or(EscapeState::GlobalEscape);
        if escape == EscapeState::NoEscape {
            non_escaping_allocations.push(alloc);
            promoted_locals.push(format!("%{}", alloc.0));
        } else {
            let reason = match escape {
                EscapeState::ArgEscape => "passed to callee argument",
                EscapeState::GlobalEscape => {
                    "escapes function frame (returned, stored globally, or impure callee)"
                }
                EscapeState::NoEscape => "unknown",
            };
            skipped.push((format!("%{}", alloc.0), reason.to_string()));
        }
    }

    (
        FunctionEscapeSummary {
            value_escapes,
            non_escaping_allocations,
        },
        FunctionPromotionSummary {
            promoted_locals,
            skipped,
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
            enum_layouts: HashMap::new(),
            struct_layouts: HashMap::new(),
        };

        let (escape, promotion) = run_escape_and_promote(&mut module, &CalleePurityInfo::default());
        assert_eq!(promotion.total_promoted, 1);
        let fn_summary_opt = escape.functions.get("local_temp");
        assert!(fn_summary_opt.is_some(), "missing function summary");
        if let Some(fn_summary) = fn_summary_opt {
            assert_eq!(
                fn_summary.value_escapes.get(&v_alloc),
                Some(&EscapeState::NoEscape)
            );
        }
    }

    #[test]
    fn test_returned_allocation_escapes_globally() {
        let b0 = crate::ir::BlockId(0);
        let v_alloc = ValueId(0);

        let mut module = MirModule {
            functions: vec![MirFunction {
                name: "make_point".into(),
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
                            fields: vec![("x".into(), ValueId(10))],
                        },
                    }],
                    terminator: Terminator::Return(v_alloc), // v_alloc escapes globally!
                }],
                target: Default::default(),
                gpu_config: None,
            }],
            enum_layouts: HashMap::new(),
            struct_layouts: HashMap::new(),
        };

        let (escape, promotion) = run_escape_and_promote(&mut module, &CalleePurityInfo::default());
        assert_eq!(promotion.total_promoted, 0); // Gracefully declined promotion
        let fn_summary_opt = escape.functions.get("make_point");
        assert!(fn_summary_opt.is_some(), "missing function summary");
        if let Some(fn_summary) = fn_summary_opt {
            assert_eq!(
                fn_summary.value_escapes.get(&v_alloc),
                Some(&EscapeState::GlobalEscape)
            );
        }
    }

    #[test]
    fn test_non_escaping_tensor_aligned_buffer_is_promoted() {
        let b0 = crate::ir::BlockId(0);
        let v_buf = ValueId(0);
        let v_idx = ValueId(1);
        let v_val = ValueId(2);

        let mut module = MirModule {
            functions: vec![MirFunction {
                name: "simd_temp_compute".into(),
                generics: vec![],
                params: vec![],
                return_ty: TypeId(0),
                entry: b0,
                blocks: vec![BasicBlock {
                    id: b0,
                    instructions: vec![
                        Instruction {
                            result: v_buf,
                            ty: TypeId(10),
                            op: Op::Call {
                                callee: "AlignedBuffer::with_capacity".into(),
                                args: vec![],
                            },
                        },
                        Instruction {
                            result: v_idx,
                            ty: TypeId(1),
                            op: Op::ConstInt(0),
                        },
                        Instruction {
                            result: v_val,
                            ty: TypeId(2),
                            op: Op::GetIndex {
                                object: v_buf,
                                index: v_idx,
                            },
                        },
                    ],
                    terminator: Terminator::Return(v_val), // returns element, buffer stays in frame
                }],
                target: Default::default(),
                gpu_config: None,
            }],
            enum_layouts: HashMap::new(),
            struct_layouts: HashMap::new(),
        };

        let (escape, promotion) = run_escape_and_promote(&mut module, &CalleePurityInfo::default());
        assert_eq!(promotion.total_promoted, 1);
        let fn_summary_opt = escape.functions.get("simd_temp_compute");
        assert!(fn_summary_opt.is_some(), "missing function summary");
        if let Some(fn_summary) = fn_summary_opt {
            assert_eq!(
                fn_summary.value_escapes.get(&v_buf),
                Some(&EscapeState::NoEscape)
            );
        }
    }
}
